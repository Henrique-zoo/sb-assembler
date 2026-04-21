use crate::{
    errors::PreprocessorError,
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Parseia uma chamada de macro em formato posicional.
    ///
    /// Forma canônica esperada:
    /// - `<MacroName>`
    /// - `<MacroName> <Arg1>, <Arg2>, ...`
    pub(in crate::preprocessor) fn parse_macro_call(
        &self,
        line: &[Token],
    ) -> Result<(Symbol, Vec<Symbol>), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Extrai o identificador da macro chamada e devolve o restante da linha.
    fn parse_macro_call_name<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(Symbol, &'a [Token], Span), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Parseia os argumentos posicionais da chamada.
    fn parse_macro_call_args(&self, rest: &[Token]) -> Result<Vec<Symbol>, PreprocessorError> {
        let _ = rest;
        todo!()
    }

    /// Parseia um único argumento posicional.
    fn parse_macro_call_arg(&self, token: &Token) -> Result<Symbol, PreprocessorError> {
        let _ = token;
        todo!()
    }
}
