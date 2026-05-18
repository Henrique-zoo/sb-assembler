//! Orquestração do estágio de pré-processamento.
//!
//! Este módulo é o ponto de integração entre lexer, detectores, parsers e
//! executores semânticos do pré-processador. Ele não tenta reconhecer a
//! gramática inteira sozinho: sua responsabilidade é conduzir o fluxo por linha,
//! manter o estado necessário entre linhas e materializar a saída preprocessada
//! no formato que a montagem propriamente dita deve consumir.
//!
//! ## Papel no pipeline
//!
//! O pré-processador recebe o vetor de tokens produzido pelo lexer. O contrato
//! esperado é que esse vetor já termine com um `NewLine` real ou sintético, de
//! modo que toda entrada possa ser percorrida como [`LogicalLine`] sem precisar
//! de um token `EOF`.
//!
//! A partir daí, [`Preprocessor::process`] executa quatro passos:
//! 1. transforma sob demanda o fluxo linear de tokens em linhas lógicas;
//! 2. despacha cada linha de acordo com a seção corrente;
//! 3. aplica efeitos de diretivas (`MACRO`, `EQU`, `IF` e `SECTION`);
//! 4. devolve um [`PreprocessedProgram`] já separado em `text` e `data`.
//!
//! ## Linhas lógicas
//!
//! A unidade de trabalho deste módulo é [`LogicalLine`]. Ela separa:
//! - `content`: tokens da linha, sem a quebra;
//! - `terminator`: a quebra de linha que encerrou aquela linha.
//!
//! Preservar o terminador é importante para expansão de macros: o corpo da macro
//! tem seus próprios terminadores, mas a última linha expandida deve herdar a
//! quebra da linha de chamada para manter a forma textual estável.
//!
//! ## Despacho por seção
//!
//! O estado [`Section`] modela em qual parte do fonte o pré-processador está:
//! - `Section::None`: região antes de uma seção explícita. Nesse contexto,
//!   `MACRO` e `EQU` são registrados, mas linhas comuns não entram no programa
//!   preprocessado;
//! - `Section::Text`: região de instruções. Aqui `IF` pode consumir a próxima
//!   linha, chamadas de macro podem expandir para várias linhas e linhas comuns
//!   são emitidas para `PreprocessedProgram::text`;
//! - `Section::Data`: região de dados. As linhas são preservadas para
//!   `PreprocessedProgram::data` sem expansão de macro ou interpretação de `IF`;
//!   aliases `EQU` em operandos de `CONST`/`SPACE` são materializados nessa
//!   fronteira de emissão.
//!
//! `SECTION TEXT` e `SECTION DATA` são tratados antes do despacho por seção
//! porque sua função é justamente alterar esse contexto. Essas diretivas não
//! aparecem como linhas emitidas; elas apenas controlam para qual vetor as
//! linhas seguintes serão acumuladas.
//!
//! ## Detecção, parsing e execução
//!
//! A orquestração por linha é deliberadamente dividida em três camadas:
//! - [`crate::preprocessor::detection`]: detectores `looks_like_*` fazem triagem
//!   permissiva. Eles decidem se uma linha parece uma tentativa de diretiva ou
//!   chamada de macro;
//! - [`crate::preprocessor::parser`]: parsers validam a forma sintática e
//!   constroem nós de IR com `NodeId`/`Span`;
//! - [`crate::preprocessor::execute`]: executores aplicam efeitos semânticos,
//!   como registrar uma macro, resolver `EQU`, avaliar `IF` ou expandir uma
//!   chamada.
//!
//! Essa separação evita que uma linha malformada seja tratada como linha comum
//! cedo demais. Primeiro o detector reconhece a intenção, depois o parser
//! produz o diagnóstico sintático específico.
//!
//! ## Diretivas multilinha
//!
//! Diretivas que dependem da linha seguinte recebem acesso ao iterador de
//! [`LogicalLine`]. Isso faz o dono da diretiva consumir o bloco que lhe
//! pertence:
//! - `MACRO ... ENDMACRO` é consumida pela execução do cabeçalho de macro;
//! - `IF <cond>` consome imediatamente a próxima linha lógica quando precisa
//!   decidir se ela será emitida.
//!
//! Assim, o avanço do iterador representa o progresso real do pré-processamento,
//! sem um estado global extra para "estou dentro de macro" ou "a próxima linha
//! pertence ao IF".
//!
//! ## Forma da saída
//!
//! A saída não é mais um fluxo textual único. O resultado de sucesso é:
//! ```ignore
//! PreprocessedProgram {
//!     text: Vec<LogicalLine>,
//!     data: Vec<LogicalLine>,
//! }
//! ```
//!
//! Quando esse resultado precisa voltar a ser renderizado como texto `.pre`, a
//! etapa de renderização deve reintroduzir os cabeçalhos de seção:
//! ```asm
//! SECTION TEXT
//! LOAD ZERO
//! STOP
//! SECTION DATA
//! ZERO CONST 0
//! ```
//!
//! ## Política de erros
//!
//! O pré-processador acumula diagnósticos em vez de interromper no primeiro
//! erro. Cada linha retorna seus efeitos por meio de [`LineProcessResult`]:
//! linhas emitidas em `output` e diagnósticos em `errors`. Ao final,
//! [`Preprocessor::process`] devolve:
//! - `Ok(PreprocessedProgram)` quando não houve erros;
//! - `Err(Vec<PreprocessorError>)` quando um ou mais problemas foram encontrados.

mod detection;
mod execute;
mod ir;
mod parser;
mod types;

pub(in crate::preprocessor) use types::LogicalLineIter;
pub(crate) use types::{LogicalLine, PreprocessedProgram, Preprocessor};

use std::collections::HashMap;

use crate::{
    errors::{PreprocessorError, PreprocessorErrorKind},
    interner::Interner,
    language::LanguageSymbols,
    lexer::{Span, Token},
    preprocessor::{ir::NodeId, types::Section},
};

use self::types::IntoLogicalLines;

/// Efeitos produzidos ao processar uma linha lógica.
///
/// O resultado é plural porque uma linha de entrada não tem relação 1:1 com a
/// saída:
/// - diretivas como `SECTION`, `MACRO`, `EQU` e `IF` falso podem emitir zero
///   linhas;
/// - linhas comuns em `TEXT`/`DATA` emitem uma linha;
/// - chamadas de macro podem emitir várias linhas.
///
/// O orquestrador agrega esses efeitos no final de cada iteração, anexando as
/// linhas emitidas à seção corrente e acumulando os diagnósticos.
struct LineProcessResult {
    /// Linhas produzidas pela linha lógica processada.
    output: Vec<LogicalLine>,
    /// Diagnósticos coletados durante detecção, parsing ou execução.
    errors: Vec<PreprocessorError>,
}

impl<'language> Preprocessor<'language> {
    /// Registra o `Span` de um nó da IR e retorna seu identificador.
    ///
    /// O pré-processador guarda spans em uma tabela lateral para que as
    /// estruturas da IR carreguem apenas um [`NodeId`]. Isso evita duplicar
    /// spans em todos os nós e mantém uma fonte única para diagnósticos.
    ///
    /// # Parâmetros
    /// - `span`: intervalo de fonte correspondente ao nó recém-construído.
    ///
    /// # Retorno
    /// - [`NodeId`] estável durante toda a execução do pré-processador.
    ///
    /// # Efeitos colaterais
    /// - adiciona `span` ao final de `self.node_spans`;
    /// - o índice usado no [`NodeId`] é a posição recém-alocada nessa tabela.
    ///
    /// # Uso no pipeline
    /// Parsers chamam esta função ao construir nós como `MacroHeader`,
    /// `MacroCall`, `EquDecl`, `IfDecl` e `SectionDecl`. Etapas posteriores
    /// podem chamar [`Self::span_of_node`] para recuperar o span original.
    fn alloc_node_id(&mut self, span: Span) -> NodeId {
        let id = NodeId(self.node_spans.len() as u32);
        self.node_spans.push(span);
        id
    }

    /// Resolve o `Span` associado a um `NodeId`.
    ///
    /// # Parâmetros
    /// - `node_id`: identificador retornado por [`Self::alloc_node_id`].
    ///
    /// # Retorno
    /// - o [`Span`] armazenado na tabela lateral;
    /// - [`Span::default`] se o identificador estiver fora dos limites.
    ///
    /// # Observações
    /// O fallback com `Span::default()` é defensivo. Em um fluxo válido, todo
    /// `NodeId` carregado pela IR deve ter sido produzido por
    /// [`Self::alloc_node_id`] no mesmo `Preprocessor`.
    fn span_of_node(&self, node_id: NodeId) -> Span {
        self.node_spans
            .get(node_id.0 as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Cria uma instância de `Preprocessor` pronta para uso.
    ///
    /// A construção prepara o estado necessário para um fluxo completo de
    /// pré-processamento: tabelas semânticas vazias, seção inicial indefinida,
    /// vocabulário compartilhado e tabela lateral de spans vazia.
    ///
    /// # Parâmetros
    /// - `language_symbols`: vocabulário internado da linguagem, compartilhado
    ///   com os demais estágios para comparação por [`crate::interner::Symbol`].
    ///
    /// # Estado inicial
    /// - `macros`: vazio;
    /// - `equs`: vazio;
    /// - `current_section`: [`Section::None`];
    /// - `node_spans`: vazio.
    ///
    /// # Efeitos colaterais
    /// Não interna novos símbolos. O vocabulário fixo já deve ter sido criado em
    /// [`LanguageSymbols::new`].
    ///
    /// # Retorno
    /// - `Preprocessor` inicializado e pronto para receber tokens em
    ///   [`Self::process`].
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let language_symbols = LanguageSymbols::new(&mut interner);
    /// let preprocessor = Preprocessor::new(&language_symbols);
    /// ```
    pub fn new(language_symbols: &'language LanguageSymbols) -> Self {
        Self {
            macros: HashMap::new(),
            equs: HashMap::new(),
            current_section: Section::None,
            language_symbols,
            node_spans: Vec::new(),
        }
    }

    /// Executa o pré-processamento sobre os tokens produzidos pelo lexer.
    ///
    /// Este é o orquestrador do módulo. Ele transforma tokens em linhas
    /// lógicas, mantém o contexto de seção, permite que diretivas consumam
    /// linhas futuras e separa a saída montável em `text` e `data`.
    ///
    /// # Parâmetros
    /// - `tokens`: fluxo completo de tokens produzido pelo lexer;
    /// - `interner`: tabela de símbolos compartilhada com as etapas anteriores
    ///   e usada para resolver símbolos durante execução semântica.
    ///
    /// # Contrato de entrada
    /// - o lexer deve garantir uma quebra `NewLine` ao final, real ou sintética;
    /// - os tokens já devem estar internados no mesmo [`Interner`] recebido por
    ///   esta função.
    ///
    /// # Fluxo
    /// 1. `tokens.into_logical_lines()` agrupa tokens por linha sob demanda.
    /// 2. Linhas `SECTION TEXT`/`SECTION DATA` atualizam `current_section`.
    /// 3. As demais linhas são despachadas para o handler da seção corrente.
    /// 4. Ao emitir para `DATA`, aliases `EQU` em operandos de `CONST`/`SPACE`
    ///    são materializados como tokens numéricos; aliases inexistentes geram
    ///    erro semântico.
    /// 5. As linhas emitidas são anexadas a [`PreprocessedProgram::text`] ou
    ///    [`PreprocessedProgram::data`].
    ///
    /// # Retorno
    /// - `Ok(PreprocessedProgram { text, data })` quando nenhum erro foi
    ///   acumulado;
    /// - `Err(errors)` quando um ou mais diagnósticos foram encontrados.
    ///
    /// # Observações
    /// Diretivas próprias do pré-processador não são preservadas como linhas na
    /// saída. `SECTION` controla roteamento, `MACRO` registra definições, `EQU`
    /// registra aliases e `IF` decide se a próxima linha deve ser emitida.
    ///
    /// # Efeitos colaterais
    /// Atualiza o estado interno do pré-processador:
    /// - registra macros em `self.macros`;
    /// - registra aliases em `self.equs`;
    /// - altera `self.current_section` quando encontra `SECTION`;
    /// - adiciona spans de nós em `self.node_spans`.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let language_symbols = LanguageSymbols::new(&mut interner);
    /// let mut preprocessor = Preprocessor::new(&language_symbols);
    ///
    /// let add = interner.entry("ADD").or_insert();
    /// let tokens = vec![
    ///     Token::new(TokenKind::Ident(add), Span { pos: 0, line: 1, column: 1, len: 3 }),
    ///     Token::new(TokenKind::NewLine, Span { pos: 3, line: 1, column: 4, len: 1 }),
    /// ];
    ///
    /// let result = preprocessor.process(tokens, &mut interner);
    /// assert!(result.is_ok());
    /// ```
    pub fn process(
        &mut self,
        tokens: Vec<Token>,
        interner: &mut Interner,
    ) -> Result<PreprocessedProgram, Vec<PreprocessorError>> {
        let mut text_lines = Vec::new();
        let mut data_lines = Vec::new();
        let mut errors = Vec::new();
        let mut lines = tokens.into_logical_lines().peekable();

        while let Some(logical_line) = lines.next() {
            if logical_line.content.is_empty() {
                continue;
            }

            if self.looks_like_section_line(&logical_line.content) {
                match self.parse_section_line(&logical_line.content) {
                    Ok(section_decl) => self.execute_section_directive(section_decl),
                    Err(err) => errors.push(err),
                }
            } else {
                match self.current_section {
                    Section::None => {
                        let (
                            LineProcessResult {
                                output,
                                errors: line_errors,
                            },
                            span,
                        ) = self.process_none_section_line(logical_line, &mut lines, interner);
                        if !output.is_empty() {
                            errors.push(PreprocessorError {
                                kind: PreprocessorErrorKind::UnknownLineSyntax,
                                span: span.unwrap(),
                            })
                        }
                        errors.extend(line_errors);
                    }
                    Section::Text => {
                        let LineProcessResult {
                            output,
                            errors: line_errors,
                        } = self.process_text_section_line(logical_line, &mut lines, interner);
                        text_lines.extend(output);
                        errors.extend(line_errors);
                    }
                    Section::Data => {
                        let LineProcessResult {
                            output,
                            errors: line_errors,
                        } = self.process_data_section_line(logical_line);
                        data_lines.extend(output);
                        errors.extend(line_errors);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(PreprocessedProgram {
                text: text_lines,
                data: data_lines,
            })
        } else {
            Err(errors)
        }
    }

    /// Processa uma linha enquanto o pré-processador está fora de uma seção.
    ///
    /// Nesse contexto, o fonte ainda não entrou em `SECTION TEXT` nem em
    /// `SECTION DATA`. O pré-processador permite declarações globais (`MACRO` e
    /// `EQU`), mas linhas comuns não entram no programa preprocessado porque
    /// ainda não há seção montável ativa.
    ///
    /// Formas reconhecidas:
    /// ```ignore
    /// NOME: MACRO &ARG
    ///     ; corpo consumido até ENDMACRO
    /// ENDMACRO
    ///
    /// CONST EQU 10
    /// ```
    ///
    /// # Parâmetros
    /// - `logical_line`: linha atual, sem `NewLine` em `content`;
    /// - `lines`: iterador das linhas seguintes, usado por `MACRO` para
    ///   consumir o corpo até `ENDMACRO`;
    /// - `interner`: tabela usada para normalizar e resolver valores de `EQU`.
    ///
    /// # Retorno
    /// - `output` vazio para `MACRO` e `EQU` válidos;
    /// - `output` com a própria linha quando ela não ativa nenhum fluxo
    ///   reconhecido;
    /// - `errors` com falhas sintáticas ou semânticas acumuladas.
    ///
    /// # Efeitos colaterais
    /// - `MACRO` válida registra uma definição em `self.macros`;
    /// - `EQU` válido registra um alias em `self.equs`;
    /// - parsers podem alocar spans em `self.node_spans`.
    ///
    /// # Observação
    /// Embora uma linha comum possa ser retornada em `output`, o orquestrador em
    /// [`Self::process`] descarta emissões enquanto `current_section` ainda é
    /// [`Section::None`].
    fn process_none_section_line(
        &mut self,
        logical_line: LogicalLine,
        lines: &mut LogicalLineIter,
        interner: &mut Interner,
    ) -> (LineProcessResult, Option<Span>) {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let line = logical_line.content.as_slice();

        if self.looks_like_macro_header(line) {
            let Some(macro_header) = self
                .parse_macro_header(line)
                .map_err(|err| errors.push(err))
                .ok()
            else {
                return (LineProcessResult { output, errors }, None);
            };

            if let Err(err) = self.execute_macro_header(macro_header, lines) {
                errors.push(err);
            }

            return (LineProcessResult { output, errors }, None);
        }

        if self.looks_like_equ_line(line) {
            let Some(equ_decl) = self
                .parse_equ_line(line)
                .map_err(|err| errors.push(err))
                .ok()
            else {
                return (LineProcessResult { output, errors }, None);
            };

            if let Err(err) = self.execute_equ_directive(equ_decl, interner) {
                errors.push(err);
            }

            return (LineProcessResult { output, errors }, None);
        }

        let span = logical_line.span();

        output.push(logical_line);
        (LineProcessResult { output, errors }, Some(span))
    }

    /// Processa uma linha dentro de `SECTION TEXT`.
    ///
    /// Esta é a região de instruções do programa. Aqui o pré-processador aplica
    /// controle condicional, expande chamadas de macro e preserva instruções
    /// comuns para a etapa de montagem.
    ///
    /// Formas reconhecidas:
    /// ```asm
    /// IF FLAG
    /// ADD VALUE
    ///
    /// MACRO_NAME ARG1, ARG2
    ///
    /// LOAD VALUE
    /// ```
    ///
    /// # Parâmetros
    /// - `logical_line`: linha atual de `SECTION TEXT`;
    /// - `lines`: iterador das próximas linhas, usado por `IF` para consumir a
    ///   linha controlada pela condição;
    /// - `interner`: tabela usada para resolver símbolos, números e aliases
    ///   `EQU` durante avaliação de `IF`.
    ///
    /// # Retorno
    /// - `output` vazio quando `IF` avalia falso;
    /// - `output` com uma linha quando a entrada é instrução comum ou `IF`
    ///   avalia verdadeiro;
    /// - `output` com várias linhas quando há expansão de macro;
    /// - `errors` com diagnósticos de `IF`, chamada de macro ou `ENDMACRO`
    ///   inesperado.
    ///
    /// # Efeitos colaterais
    /// - `IF` pode avançar o iterador `lines`;
    /// - chamadas de macro consultam `self.macros`;
    /// - parsers podem alocar spans em `self.node_spans`.
    ///
    /// # Contrato de terminador
    /// Quando uma chamada de macro expande para várias linhas, a última linha
    /// emitida recebe o `terminator` da linha de chamada. Isso preserva a
    /// fronteira textual observada pelo restante do pipeline.
    fn process_text_section_line(
        &mut self,
        logical_line: LogicalLine,
        lines: &mut LogicalLineIter,
        interner: &mut Interner,
    ) -> LineProcessResult {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let LogicalLine {
            content,
            terminator,
        } = logical_line;
        let line = content.as_slice();

        if self.looks_like_if_line(line) {
            let Some(if_decl) = self
                .parse_if_line(line)
                .map_err(|err| errors.push(err))
                .ok()
            else {
                return LineProcessResult { output, errors };
            };

            match self.execute_if_directive(if_decl, lines, interner) {
                Ok(Some(emitted_tokens)) => output.push(LogicalLine {
                    content: emitted_tokens,
                    terminator,
                }),
                Ok(None) => {}
                Err(err) => errors.push(err),
            }
            return LineProcessResult { output, errors };
        }

        if self.looks_like_endmacro_line(line) {
            errors.push(PreprocessorError {
                kind: PreprocessorErrorKind::UnexpectedEndMacro,
                span: line[0].span,
            })
        }

        if self.looks_like_macro_call(line) {
            match self.parse_macro_call(line) {
                Ok(macro_call) => match self.execute_macro_call(macro_call) {
                    Ok(mut expanded_lines) => {
                        if let Some(last_line) = expanded_lines.last_mut() {
                            last_line.terminator = terminator;
                        } else {
                            expanded_lines.push(LogicalLine {
                                content: Vec::new(),
                                terminator,
                            });
                        }

                        output.extend(expanded_lines);
                        return LineProcessResult { output, errors };
                    }
                    Err(err) => errors.push(err),
                },
                Err(err) => {
                    errors.push(err);
                }
            }
        }

        output.push(LogicalLine {
            content,
            terminator,
        });
        LineProcessResult { output, errors }
    }

    /// Processa uma linha dentro de `SECTION DATA`.
    ///
    /// A seção de dados é tratada como conteúdo montável bruto neste estágio.
    /// O pré-processador não interpreta `IF` nem expande chamadas de macro.
    /// Nesta fase, aliases `EQU` usados em operandos de `CONST`/`SPACE` são
    /// materializados (substituídos por tokens numéricos) antes de a linha ser
    /// emitida para `PreprocessedProgram::data`.
    ///
    /// Forma típica:
    /// ```ignore
    /// VALUE SPACE
    /// TABLE CONST 4
    /// ```
    ///
    /// # Parâmetros
    /// - `logical_line`: a linha atual de `SECTION DATA`. A implementação pode
    ///   modificar o conteúdo desta linha ao materializar aliases `EQU`.
    ///
    /// # Retorno
    /// - [`LineProcessResult`] em que `output` contém a linha (possivelmente
    ///   alterada) a ser emitida para `PreprocessedProgram::data`;
    /// - `errors` pode conter diagnósticos gerados durante a materialização de
    ///   `EQU` (por exemplo, alias inexistente ou erro semântico).
    ///
    /// # Efeitos colaterais
    /// - pode alterar `logical_line.content` ao substituir aliases `EQU`;
    /// - não altera `self.current_section` nem registra definições; apenas
    ///   prepara a linha para emissão em `data`.
    fn process_data_section_line(&mut self, mut logical_line: LogicalLine) -> LineProcessResult {
        let mut output = Vec::new();
        let mut errors = Vec::new();

        if self.looks_like_equ_use(&logical_line.content) {
            if let Err(err) = self.replace_data_equ_use(&mut logical_line.content) {
                errors.push(err)
            }
        }

        output.push(logical_line);

        LineProcessResult { output, errors }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
