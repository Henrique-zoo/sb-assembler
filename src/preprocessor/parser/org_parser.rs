use crate::{
    errors::PreprocessorError,
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `ORG`.
    ///
    /// Forma canônica esperada:
    /// - `ORG <Valor>`
    ///
    /// Regras sintáticas aceitas para `<Valor>`:
    /// - `Number` literal;
    /// - `Ident` (candidato a alias definido por `EQU`).
    ///
    /// Importante:
    /// - este parser valida apenas forma/tokenização;
    /// - ele **não** resolve nem valida semanticamente o `Ident` contra a
    ///   tabela de `EQU`. Essa responsabilidade é do estágio de execução.
    ///
    /// Retorno planejado:
    /// - `Ok(value)`, onde `value` representa o `Symbol` internado do operando
    ///   (seja número, seja identificador).
    pub(in crate::preprocessor) fn parse_org_line(
        &self,
        line: &[Token],
    ) -> Result<Symbol, PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Consome a keyword `ORG` no início da diretiva.
    fn consume_org_keyword<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(&'a [Token], Span), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Parseia o operando de `ORG` (`Number` ou `Ident`).
    ///
    /// Esta etapa é estritamente sintática:
    /// - garante que existe exatamente um operando válido;
    /// - não verifica se `Ident` existe em `self.equs`.
    fn parse_org_value(
        &self,
        rest: &[Token],
        fallback_span: Span,
    ) -> Result<Symbol, PreprocessorError> {
        let _ = (rest, fallback_span);
        todo!()
    }
}
