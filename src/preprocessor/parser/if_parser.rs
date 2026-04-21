use crate::{
    errors::PreprocessorError,
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `IF`.
    ///
    /// Forma canônica esperada:
    /// - `IF <Expressao>`
    ///
    /// Retorno planejado:
    /// - `Ok(cond)`, onde `cond` representa o símbolo da condição.
    pub(in crate::preprocessor) fn parse_if_line(&self, line: &[Token]) -> Result<Symbol, PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Consome a keyword `IF` no início da diretiva.
    fn consume_if_keyword<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(&'a [Token], Span), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Parseia a expressão/condição de uma diretiva `IF`.
    fn parse_if_condition(
        &self,
        rest: &[Token],
        fallback_span: Span,
    ) -> Result<Symbol, PreprocessorError> {
        let _ = (rest, fallback_span);
        todo!()
    }
}
