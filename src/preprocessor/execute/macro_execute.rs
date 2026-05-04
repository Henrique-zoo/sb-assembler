use std::{iter::Peekable, vec::IntoIter};

use crate::{
    errors::{PreprocessorError, PreprocessorErrorKind},
    interner::Interner,
    lexer::Token,
    preprocessor::{
        Preprocessor,
        ir::{Macro, MacroHeader},
        types::LogicalLine,
    },
};

impl Preprocessor {
    /// Aplica os efeitos semânticos de um cabeçalho `MACRO` já parseado.
    ///
    /// A função recebe o iterador de linhas lógicas e consome o bloco da macro
    /// até encontrar uma linha candidata a `ENDMACRO`.
    ///
    /// Durante o consumo:
    /// - linhas intermediárias são acumuladas no body da macro;
    /// - os terminadores das linhas consumidas são preservados em `output`.
    pub(in crate::preprocessor) fn execute_macro_header(
        &mut self,
        macro_header: MacroHeader,
        lines: &mut Peekable<IntoIter<LogicalLine>>,
        output: &mut Vec<Token>,
    ) -> Result<(), PreprocessorError> {
        let mut body = Vec::new();
        let mut unterminated_span = macro_header.span;

        while let Some(logical_line) = lines.next() {
            if self.looks_like_endmacro_line(&logical_line.content) {
                output.push(logical_line.terminator);
                self.parse_endmacro_line(&logical_line.content)?;

                self.macros.insert(
                    macro_header.name,
                    Macro {
                        header: macro_header,
                        body,
                    },
                );
                return Ok(());
            }

            if !logical_line.content.is_empty() {
                unterminated_span = logical_line.content[0].span;
                body.push(logical_line.content);
            }

            output.push(logical_line.terminator);
        }

        Err(PreprocessorError {
            kind: PreprocessorErrorKind::UnterminatedMacro,
            span: unterminated_span,
        })
    }

    /// Tenta expandir uma chamada de macro em uma ou mais linhas de tokens.
    ///
    /// Retorna:
    /// - `Some(expanded_lines)` quando a linha representa chamada de macro;
    /// - `None` quando não é chamada de macro.
    pub(in crate::preprocessor) fn expand_macro_call(
        &mut self,
        line: &[Token],
        interner: &mut Interner,
        errors: &mut Vec<PreprocessorError>,
    ) -> Option<Vec<Vec<Token>>> {
        let _ = (line, interner, errors);
        todo!()
    }
}
