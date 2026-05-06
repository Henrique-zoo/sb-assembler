use crate::{
    errors::{
        DirectiveKind, IfDirectiveSyntaticErrorKind, PreprocessorError,
        PreprocessorErrorKind::InvalidIfDirectiveSyntatic,
    },
    lexer::{Span, Token},
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
    /// - `Ok(IfDecl { node_id, cond })` quando a linha é sintaticamente válida.
    ///
    /// Erros:
    /// - propaga falhas de qualquer etapa do pipeline (keyword ausente/
    ///   inesperada, condição ausente/tipo inválido ou trailing tokens).
    ///
    /// Efeito colateral:
    /// - nenhum. A função apenas valida e extrai estrutura para IR.
    pub(in crate::preprocessor) fn parse_if_line(
        &mut self,
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
        let node_id = self.alloc_node_id(span);

        Ok(IfDecl {
            node_id,
            cond: operand,
        })
    }

    /// Parseia a expressão/condição de uma diretiva `IF`.
    ///
    /// Formas aceitas:
    /// - `IF <Number>`;
    /// - `IF <Ident>`;
    /// - `IF +<Number>`;
    /// - `IF -<Number>`.
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
    /// - `InvalidIfDirectiveSyntatic(MissingCondition)` quando não há token de
    ///   condição;
    /// - `InvalidIfDirectiveSyntatic(InvalidConditionType)` quando o token da
    ///   condição não é `Number`, `Ident`, `+Number` ou `-Number`.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_if_condition<'a>(
        &mut self,
        line: &'a [Token],
        fallback_span: Span,
    ) -> Result<(Operand, &'a [Token], Span), PreprocessorError> {
        self.parse_operand(line, fallback_span)
            .map_err(|err| match err.kind {
                super::OperandParseErrorKind::Missing => Self::if_directive_sintatic_error(
                    IfDirectiveSyntaticErrorKind::MissingCondition,
                    err.span,
                ),
                super::OperandParseErrorKind::InvalidType => Self::if_directive_sintatic_error(
                    IfDirectiveSyntaticErrorKind::InvalidConditionType,
                    err.span,
                ),
            })
    }

    /// Constrói erro específico de diretiva `IF` já envelopado em
    /// [`crate::errors::PreprocessorErrorKind::InvalidIfDirectiveSyntatic`].
    ///
    /// Este helper centraliza o boilerplate de criação de erros da família `IF`,
    /// preservando o `span` informado pelo chamador.
    fn if_directive_sintatic_error(
        kind: IfDirectiveSyntaticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: InvalidIfDirectiveSyntatic(kind),
            span,
        }
    }
}
