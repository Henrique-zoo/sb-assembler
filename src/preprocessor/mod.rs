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
mod parser;
mod types;

pub(crate) use types::{Keywords, Preprocessor};

use std::{collections::HashMap, iter::Peekable, vec::IntoIter};

use crate::{
    errors::PreprocessorError,
    interner::Interner,
    lexer::{Token, TokenKind},
    preprocessor::types::{FixedSymbols, LogicalLine, Section},
};

/// Efeitos produzidos pelo processamento de uma única linha lógica.
struct LineProcessResult {
    output: Vec<Token>,
    errors: Vec<PreprocessorError>,
}

impl Preprocessor {
    /// Cria uma instância de `Preprocessor` pronta para uso.
    ///
    /// A construção inicializa a seção atual como `Section::None`, cria tabelas
    /// vazias para macros e `EQU`, e interna as keywords do pré-processador
    /// para comparações eficientes por símbolo.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let preprocessor = Preprocessor::new(&mut interner);
    /// ```
    pub fn new(interner: &mut Interner) -> Self {
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
        };

        Self {
            macros: HashMap::new(),
            equs: HashMap::new(),
            current_section: Section::None,
            keywords,
            fixed_symbols,
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
    /// let mut preprocessor = Preprocessor::new(&mut interner);
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
    ) -> Result<Vec<Token>, Vec<PreprocessorError>> {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let mut lines = Self::collect_logical_lines(tokens).into_iter().peekable();

        while let Some(logical_line) = lines.next() {
            let line_result = self.process_line(logical_line, &mut lines, interner);
            output.extend(line_result.output);
            errors.extend(line_result.errors);
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

        if line.is_empty() {
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        if self.looks_like_macro_header(line) {
            output.push(terminator);

            let Some(macro_header) = self
                .parse_macro_header(line)
                .map_err(|err| errors.push(err))
                .ok()
            else {
                return LineProcessResult { output, errors };
            };

            if let Err(err) = self.execute_macro_header(macro_header, lines, &mut output) {
                errors.push(err);
            }
            return LineProcessResult { output, errors };
        }

        if self.looks_like_equ_line(line) {
            if let Err(err) = self.process_equ(line) {
                errors.push(err);
            }
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        if self.looks_like_if_line(line) {
            if let Err(err) = self.process_if(line, &mut output) {
                errors.push(err);
            }
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        if self.looks_like_text_line(line) {
            match self.parse_text_line(line) {
                Ok(section_decl) => {
                    self.current_section = section_decl.section;
                }
                Err(err) => {
                    errors.push(err);
                }
            }
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        if self.looks_like_data_line(line) {
            match self.parse_data_line(line) {
                Ok(section_decl) => {
                    self.current_section = section_decl.section;
                }
                Err(err) => {
                    errors.push(err);
                }
            }
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        if let Some(expanded) = self.expand_macro_call(line, interner, &mut errors) {
            for expanded_line in expanded {
                output.extend(expanded_line);
            }
            output.push(terminator);
            return LineProcessResult { output, errors };
        }

        output.extend_from_slice(line);
        output.push(terminator);
        LineProcessResult { output, errors }
    }

    /// Agrupa o fluxo linear de tokens em [`LogicalLine`]s.
    ///
    /// Cada linha lógica é finalizada quando encontra `NewLine` ou `Eof`.
    /// Nesse ponto:
    /// - `content` recebe os tokens acumulados antes do terminador;
    /// - `terminator` guarda o separador original da linha.
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
            if matches!(&token.kind, TokenKind::NewLine | TokenKind::Eof) {
                lines.push(LogicalLine {
                    content: std::mem::take(&mut current_line),
                    terminator: token,
                });
            } else {
                current_line.push(token);
            }
        }

        lines
    }
}
