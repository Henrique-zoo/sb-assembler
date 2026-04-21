use crate::{
    errors::PreprocessorError,
    lexer::{Span, Token},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Parseia uma linha do corpo de macro.
    ///
    /// Este parser trata somente sintaxe local de linha do body (ex.: forma de
    /// referência a parâmetros formais), sem aplicar efeitos de execução.
    pub(in crate::preprocessor) fn parse_macro_body_line(
        &self,
        line: &[Token],
    ) -> Result<Vec<Token>, PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Parseia a linha de encerramento do corpo (`ENDMACRO`).
    pub(in crate::preprocessor) fn parse_macro_body_terminator(
        &self,
        line: &[Token],
    ) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Valida invariantes sintáticas mínimas para uma linha de body.
    fn ensure_macro_body_line_is_valid(
        &self,
        line: &[Token],
        fallback_span: Span,
    ) -> Result<(), PreprocessorError> {
        let _ = (line, fallback_span);
        todo!()
    }

    /// Parseia referências a parâmetros formais (ex.: `&ARG`) dentro do body.
    fn parse_macro_body_param_refs(&self, line: &[Token]) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }
}
