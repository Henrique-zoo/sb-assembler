//! Orquestração do pré-processador.
//!
//! Este módulo coordena o pipeline de pré-processamento por linha:
//! 1. triagem com detectores (`detection::looks_like_*`);
//! 2. validação sintática/extração (`parser::*`);
//! 3. aplicação de efeitos (`execute::*`: registro de macro/EQU, expansão, controle de
//!    fluxo) e acumulação de erros.
//!
//! Separação de responsabilidades:
//! - [`crate::preprocessor::detection`]: reconhecimento permissivo de tentativa
//!   de diretiva;
//! - [`crate::preprocessor::parser`]: validação sintática estrita e parsing;
//! - [`crate::preprocessor::execute`]: execução semântica das diretivas;
//! - este módulo: despacho por linha e montagem do output.

mod detection;
mod execute;
mod ir;
mod number;
mod parser;
mod types;

pub(in crate::preprocessor) use number::{
    NumberParseError, parse_signed_number, parse_unsigned_number,
};
pub(crate) use types::{Keywords, LogicalLine, Preprocessor};

use std::{collections::HashMap, iter::Peekable, vec::IntoIter};

use crate::{
    errors::{PreprocessorError, PreprocessorErrorKind},
    interner::Interner,
    language::KeywordTable,
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        ir::NodeId,
        types::{FixedSymbols, Section},
    },
};

/// Efeitos produzidos pelo processamento de uma única linha lógica.
struct LineProcessResult {
    output: Vec<LogicalLine>,
    errors: Vec<PreprocessorError>,
}

impl Preprocessor {
    /// Registra um nó da IR e retorna seu `NodeId`.
    ///
    /// O `NodeId` é estável durante todo o processamento e pode ser usado para
    /// recuperar o `Span` com [`Self::span_of_node`].
    fn alloc_node_id(&mut self, span: Span) -> NodeId {
        let id = NodeId(self.node_spans.len() as u32);
        self.node_spans.push(span);
        id
    }

    /// Resolve o `Span` associado a um `NodeId`.
    ///
    /// Retorna `Span::default()` se o `NodeId` estiver fora dos limites da
    /// tabela (fallback defensivo).
    fn span_of_node(&self, node_id: NodeId) -> Span {
        self.node_spans
            .get(node_id.0 as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Cria uma instância de `Preprocessor` pronta para uso.
    ///
    /// A construção inicializa a seção atual como `Section::None`, cria tabelas
    /// vazias para macros e `EQU`, e interna as keywords do pré-processador
    /// para comparações eficientes por símbolo.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let keywords = KeywordTable::new(&mut interner);
    /// let preprocessor = Preprocessor::new(&mut interner, keywords);
    /// ```
    pub fn new(interner: &mut Interner, keyword_table: KeywordTable) -> Self {
        let keywords = Keywords {
            section_kw: interner.entry("SECTION").or_insert(),
            text_kw: interner.entry("TEXT").or_insert(),
            data_kw: interner.entry("DATA").or_insert(),
            macro_kw: interner.entry("MACRO").or_insert(),
            endmacro_kw: interner.entry("ENDMACRO").or_insert(),
            equ_kw: interner.entry("EQU").or_insert(),
            if_kw: interner.entry("IF").or_insert(),
        };

        let fixed_symbols = FixedSymbols {
            ampersand: interner.entry("&").or_insert(),
            comma: interner.entry(",").or_insert(),
            colon: interner.entry(":").or_insert(),
            plus: interner.entry("+").or_insert(),
            minus: interner.entry("-").or_insert(),
            generic_ident: interner.entry("A").or_insert(),
            generic_number: interner.entry("10").or_insert(),
        };

        Self {
            macros: HashMap::new(),
            equs: HashMap::new(),
            current_section: Section::None,
            keywords,
            keyword_table,
            fixed_symbols,
            node_spans: Vec::new(),
        }
    }

    /// Executa o pré-processamento sobre o fluxo de tokens produzido pelo lexer.
    ///
    /// A função agrupa os tokens em linhas lógicas e delega cada linha para
    /// `process_line`. Os separadores originais (`NewLine` e `Eof`) são
    /// preservados no resultado.
    ///
    /// O processamento não interrompe no primeiro problema: erros são
    /// acumulados e devolvidos juntos no final. Durante esse fluxo, o
    /// pré-processador pode alterar estado, registrar diretivas e expandir
    /// macros.
    ///
    /// Retorna `Ok(output)` quando nenhum erro foi acumulado e `Err(errors)`
    /// quando há um ou mais diagnósticos.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let keywords = KeywordTable::new(&mut interner);
    /// let mut preprocessor = Preprocessor::new(&mut interner, keywords);
    ///
    /// let add = interner.entry("ADD").or_insert();
    /// let tokens = vec![
    ///     Token::new(TokenKind::Ident(add), Span { pos: 0, line: 1, column: 1, len: 3 }),
    ///     Token::new(TokenKind::NewLine, Span { pos: 3, line: 1, column: 4, len: 1 }),
    ///     Token::new(TokenKind::Eof, Span { pos: 4, line: 2, column: 1, len: 0 }),
    /// ];
    ///
    /// let result = preprocessor.process(tokens, &mut interner);
    /// assert!(result.is_ok());
    /// ```
    pub fn process(
        &mut self,
        tokens: Vec<Token>,
        interner: &mut Interner,
    ) -> Result<Vec<LogicalLine>, Vec<PreprocessorError>> {
        let mut output = Vec::new();
        let mut cod_lines = Vec::new();
        let mut data_lines = Vec::new();
        let mut eof_line = None;
        let mut errors = Vec::new();
        let mut lines = Self::collect_logical_lines(tokens).into_iter().peekable();

        while let Some(logical_line) = lines.next() {
            let line_result = self.process_line(logical_line, &mut lines, interner);
            for emitted_line in line_result.output {
                if matches!(emitted_line.terminator.kind, TokenKind::Eof) {
                    eof_line = Some(emitted_line);
                    continue;
                }

                match self.current_section {
                    Section::None => output.push(emitted_line),
                    Section::Data => data_lines.push(emitted_line),
                    Section::Text => cod_lines.push(emitted_line),
                }
            }
            errors.extend(line_result.errors);
        }

        output.extend(cod_lines);
        output.extend(data_lines);
        if let Some(eof_line) = eof_line {
            output.push(eof_line);
        }

        if errors.is_empty() {
            Ok(output)
        } else {
            Err(errors)
        }
    }

    /// Processa uma linha lógica (sem `NewLine`/`Eof`).
    ///
    /// A função usa os detectores `looks_like_*` como triagem rápida e delega a
    /// validação/aplicação da diretiva para as rotinas correspondentes.
    ///
    /// Despacho por seção:
    /// - `SECTION TEXT`/`SECTION DATA` são tratados antes de qualquer outro
    ///   fluxo para permitir troca de contexto;
    /// - em `Section::None`, apenas tentativas de `MACRO` e `EQU` são
    ///   reconhecidas;
    /// - em `Section::Text`, apenas tentativas de `IF` e `macro call` são
    ///   reconhecidas;
    /// - em `Section::Data`, linhas são apenas repassadas para o output.
    ///
    /// Para definição de macro, `execute_macro_header` recebe o iterador de
    /// linhas e consome o bloco até `ENDMACRO`.
    ///
    /// Essa função pode atualizar tabelas internas e retorna os efeitos
    /// produzidos naquela linha (`output` emitido + erros acumulados).
    ///
    /// # Exemplo (uso interno)
    /// ```rust,ignore
    /// let mut lines = Vec::new().into_iter().peekable();
    /// let result = preprocessor.process_line(line, &mut lines, &mut interner);
    /// assert!(result.errors.is_empty());
    /// ```
    fn process_line(
        &mut self,
        logical_line: LogicalLine,
        lines: &mut Peekable<IntoIter<LogicalLine>>,
        interner: &mut Interner,
    ) -> LineProcessResult {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let line = logical_line.content.as_slice();
        let terminator = logical_line.terminator;

        if self.looks_like_text_section_line(line) {
            match self.parse_text_section_line(line) {
                Ok(section_decl) => {
                    output.extend(self.execute_section_directive(section_decl, line, terminator))
                }
                Err(err) => errors.push(err),
            }
            return LineProcessResult { output, errors };
        }

        if self.looks_like_data_section_line(line) {
            match self.parse_data_section_line(line) {
                Ok(section_decl) => {
                    output.extend(self.execute_section_directive(section_decl, line, terminator))
                }
                Err(err) => errors.push(err),
            }
            return LineProcessResult { output, errors };
        }

        if matches!(self.current_section, Section::Data) {
            output.push(LogicalLine {
                content: line.to_vec(),
                terminator,
            });
            return LineProcessResult { output, errors };
        }

        match self.current_section {
            Section::None => {
                if self.looks_like_macro_header(line) {
                    let Some(macro_header) = self
                        .parse_macro_header(line)
                        .map_err(|err| errors.push(err))
                        .ok()
                    else {
                        return LineProcessResult { output, errors };
                    };

                    match self.execute_macro_header(macro_header, lines) {
                        Ok(()) => {}
                        Err(err) => errors.push(err),
                    }
                    return LineProcessResult { output, errors };
                }

                if self.looks_like_equ_line(line) {
                    let Some(equ_decl) = self
                        .parse_equ_line(line)
                        .map_err(|err| errors.push(err))
                        .ok()
                    else {
                        return LineProcessResult { output, errors };
                    };

                    if let Err(err) = self.execute_equ_directive(equ_decl, interner) {
                        errors.push(err);
                    }
                    return LineProcessResult { output, errors };
                }
            }
            Section::Text => {
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
            }
            Section::Data => {}
        }

        output.push(LogicalLine {
            content: line.to_vec(),
            terminator,
        });
        LineProcessResult { output, errors }
    }

    /// Agrupa o fluxo linear de tokens em [`LogicalLine`]s.
    ///
    /// Cada linha lógica é finalizada quando encontra `NewLine` ou `Eof`.
    /// Nesse ponto:
    /// - `content` recebe os tokens acumulados antes do terminador;
    /// - `terminator` guarda o separador original da linha.
    ///
    /// Convenção de fronteira final:
    /// - se o arquivo termina sem `\n` após a última linha de conteúdo, a
    ///   função cria um `NewLine` sintético como terminador dessa linha;
    /// - `Eof` é sempre materializado como uma linha lógica própria
    ///   (`content` vazio, `terminator = Eof`).
    ///
    /// Preservar o `terminator` permite que as próximas etapas processem por
    /// linha sem perder a estrutura original do fonte (linhas vazias,
    /// quebras e fim de arquivo continuam explícitos no pipeline).
    ///
    /// Contrato esperado:
    /// - o lexer deve emitir `Eof`;
    /// - a função não valida sintaxe, apenas reorganiza tokens por linha.
    ///
    /// Nota:
    /// - se a entrada não contiver `NewLine`/`Eof` ao final, os tokens
    ///   remanescentes não são emitidos como linha lógica.
    fn collect_logical_lines(tokens: Vec<Token>) -> Vec<LogicalLine> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();

        for token in tokens {
            match &token.kind {
                TokenKind::NewLine => lines.push(LogicalLine {
                    content: std::mem::take(&mut current_line),
                    terminator: token,
                }),
                TokenKind::Eof => {
                    if !current_line.is_empty() {
                        lines.push(LogicalLine {
                            content: std::mem::take(&mut current_line),
                            terminator: Token::new(TokenKind::NewLine, Span::default()),
                        });
                    }
                    lines.push(LogicalLine {
                        content: vec![],
                        terminator: token,
                    })
                }
                _ => current_line.push(token),
            }
        }

        lines
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
