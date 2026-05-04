use crate::{
    errors::{
        DirectiveKind, IfDirectiveErrorKind, PreprocessorError,
        PreprocessorErrorKind::InvalidIfDirective,
    },
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        Preprocessor,
        ir::{IfDecl, Operand},
    },
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `IF`.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// IF <Expressao>
    /// ```
    ///
    /// Fluxo interno:
    /// 1. valida e consome a keyword `IF` com [`Self::consume_keyword`];
    /// 2. parseia a condição com [`Self::parse_if_condition`];
    /// 3. garante ausência de tokens extras com
    ///    [`Self::ensure_no_trailing_tokens`].
    ///
    /// Retorno:
    /// - `Ok(IfDecl { cond, span })` quando a linha é sintaticamente válida.
    ///
    /// Erros:
    /// - propaga falhas de qualquer etapa do pipeline (keyword ausente/
    ///   inesperada, condição ausente/tipo inválido ou trailing tokens).
    ///
    /// Efeito colateral:
    /// - nenhum. A função apenas valida e extrai estrutura para IR.
    pub(in crate::preprocessor) fn parse_if_line(
        &self,
        line: &[Token],
    ) -> Result<IfDecl, PreprocessorError> {
        let (tail, fallback_span) = self.consume_keyword(
            line,
            DirectiveKind::If,
            self.keywords.if_kw,
            Span::default(),
        )?;
        let (operand, tail, fallback_span) = self.parse_if_condition(tail, fallback_span)?;

        self.ensure_no_trailing_tokens(tail, DirectiveKind::If)?;
        let span = self.consumed_prefix_span(line, tail, fallback_span);

        Ok(IfDecl {
            cond: operand,
            span,
        })
    }

    /// Parseia a expressão/condição de uma diretiva `IF`.
    ///
    /// Formas aceitas:
    /// - `IF <Number>`;
    /// - `IF <Ident>`.
    ///
    /// Parâmetros:
    /// - `line`: sufixo após `IF`;
    /// - `fallback_span`: span usado quando a condição está ausente.
    ///
    /// Retorno:
    /// - `Ok((operand, tail, cond_span))`, onde:
    ///   - `operand` representa a condição (`Number` ou `Ident`);
    ///   - `tail` contém os tokens remanescentes;
    ///   - `cond_span` aponta para o token da condição.
    ///
    /// Erros:
    /// - `InvalidIfDirective(MissingCondition)` quando não há token de condição;
    /// - `InvalidIfDirective(InvalidConditionType)` quando o token da condição
    ///   não é `Number` nem `Ident`.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_if_condition<'a>(
        &self,
        line: &'a [Token],
        fallback_span: Span,
    ) -> Result<(Operand, &'a [Token], Span), PreprocessorError> {
        let (cond_token, tail) = line.split_first().ok_or_else(|| {
            Self::if_directive_error(IfDirectiveErrorKind::MissingCondition, fallback_span)
        })?;

        match cond_token.kind {
            TokenKind::Number(sym) => Ok((Operand::Number(sym), tail, cond_token.span)),
            TokenKind::Ident(sym) => Ok((Operand::Ident(sym), tail, cond_token.span)),
            _ => Err(Self::if_directive_error(
                IfDirectiveErrorKind::InvalidConditionType,
                cond_token.span,
            )),
        }
    }

    /// Constrói erro específico de diretiva `IF` já envelopado em
    /// [`crate::errors::PreprocessorErrorKind::InvalidIfDirective`].
    ///
    /// Este helper centraliza o boilerplate de criação de erros da família `IF`,
    /// preservando o `span` informado pelo chamador.
    fn if_directive_error(kind: IfDirectiveErrorKind, span: Span) -> PreprocessorError {
        PreprocessorError {
            kind: InvalidIfDirective(kind),
            span,
        }
    }
}
