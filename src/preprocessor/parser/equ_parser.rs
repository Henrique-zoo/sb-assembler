use crate::{
    errors::PreprocessorError,
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `EQU`.
    ///
    /// Forma canônica esperada:
    /// - `<Alias> EQU <Valor>`
    ///
    /// Retorno planejado:
    /// - `Ok((alias, value))`, com ambos internados em `Symbol`.
    pub(in crate::preprocessor) fn parse_equ_line(
        &self,
        line: &[Token],
    ) -> Result<(Symbol, Symbol), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Extrai e valida o alias inicial da diretiva `EQU`.
    fn parse_equ_alias<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(Symbol, &'a [Token], Span), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Garante que não há `:` após o alias em uma diretiva `EQU`.
    fn ensure_no_colon_after_equ_alias<'a>(
        &self,
        rest: &'a [Token],
    ) -> Result<&'a [Token], PreprocessorError> {
        let _ = rest;
        todo!()
    }

    /// Consome a keyword `EQU` após o alias.
    fn consume_equ_keyword<'a>(
        &self,
        rest: &'a [Token],
        fallback_span: Span,
    ) -> Result<&'a [Token], PreprocessorError> {
        let _ = (rest, fallback_span);
        todo!()
    }

    /// Parseia o valor da diretiva `EQU`.
    fn parse_equ_value(&self, rest: &[Token]) -> Result<Symbol, PreprocessorError> {
        let _ = rest;
        todo!()
    }
}
