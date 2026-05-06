//! Parser da diretiva `EQU`.
//!
//! Este módulo concentra a validação sintática de aliases do tipo:
//! ```ignore
//! <Alias> EQU <Valor>
//! ```
//!
//! Responsabilidades:
//! - validar e extrair o alias inicial;
//! - rejeitar padrões clássicos inválidos (ex.: `:` após alias);
//! - consumir a keyword `EQU`;
//! - parsear o valor aceito (`Ident` ou `Number`);
//! - garantir ausência de tokens extras.
//!
//! Saída:
//! - retorna [`crate::preprocessor::ir::EquDecl`], mantendo parser e execução
//!   desacoplados.

use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, EquDirectiveSyntaticErrorKind, PreprocessorError,
        PreprocessorErrorKind::InvalidEquDirectiveSyntatic,
    },
    interner::Symbol,
    lexer::{
        Span, Token,
        TokenKind::{self, Ident},
    },
    preprocessor::{
        Preprocessor,
        ir::{EquDecl, Operand},
    },
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `EQU`.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// <Alias> EQU <Valor>
    /// ```
    ///
    /// Fluxo interno:
    /// 1. extrai alias com [`Self::parse_equ_alias`];
    /// 2. bloqueia `:` na posição atual com [`Self::ensure_token_is_not`];
    /// 3. consome `EQU` com [`Self::consume_keyword`];
    /// 4. parseia valor com [`Self::parse_equ_value`];
    /// 5. garante ausência de tokens extras com
    ///    [`Self::ensure_no_trailing_tokens`];
    /// 6. consolida `span` final com [`Self::consumed_prefix_span`].
    ///
    /// Retorno:
    /// - `Ok(EquDecl { node_id, alias, value })` quando a diretiva está correta.
    ///
    /// Erros:
    /// - propaga falhas de alias ausente/inválido;
    /// - propaga erro de token proibido (`ForbiddenToken`) para `:`;
    /// - propaga erros de keyword `EQU` ausente/inesperada;
    /// - propaga erro de valor ausente/tipo inválido;
    /// - propaga erro de trailing tokens.
    ///
    /// Efeito colateral:
    /// - nenhum. Apenas valida e monta IR.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let decl = preprocessor.parse_equ_line(tokens)?;
    /// // Exemplo de forma válida:
    /// // COUNT EQU 10
    /// ```
    pub(in crate::preprocessor) fn parse_equ_line(
        &mut self,
        line: &[Token],
    ) -> Result<EquDecl, PreprocessorError> {
        let (alias, tail, fallback_span) = self.parse_equ_alias(line)?;
        self.ensure_token_is_not(tail, TokenKind::Colon, DirectiveKind::Equ)?;

        let (tail, fallback_span) = self.consume_keyword(
            tail,
            DirectiveKind::Equ,
            self.keywords.equ_kw,
            fallback_span,
        )?;

        let (value, tail, fallback_span) = self.parse_equ_value(tail, fallback_span)?;

        self.ensure_no_trailing_tokens(tail, DirectiveKind::Equ)?;
        let span = self.consumed_prefix_span(line, tail, fallback_span);
        let node_id = self.alloc_node_id(span);

        Ok(EquDecl {
            node_id,
            alias,
            value,
        })
    }

    /// Extrai e valida o alias inicial da diretiva `EQU`.
    ///
    /// Forma esperada (prefixo):
    /// ```ignore
    /// <Alias> ...
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: linha da diretiva ainda não consumida.
    ///
    /// Retorno:
    /// - `Ok((alias, tail, alias_span))`, onde:
    ///   - `alias` é o símbolo do identificador inicial;
    ///   - `tail` é o sufixo após o alias;
    ///   - `alias_span` é o span do token de alias.
    ///
    /// Erros:
    /// - `MissingToken` quando a linha está vazia;
    /// - `UnexpectedToken` quando o primeiro token não é `Ident`.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_equ_alias<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(Symbol, &'a [Token], Span), PreprocessorError> {
        let (alias_token, tail) = line.split_first().ok_or_else(|| {
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive: DirectiveKind::Equ,
                    token_missed: TokenKind::Ident(self.keywords.equ_kw),
                },
                Span::default(),
            )
        })?;

        if let Ident(sym) = alias_token.kind {
            Ok((sym, tail, alias_token.span))
        } else {
            Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::Equ,
                    expected_token: TokenKind::Ident(self.keywords.equ_kw),
                },
                alias_token.span,
            ))
        }
    }

    /// Parseia o valor da diretiva `EQU`.
    ///
    /// Formas aceitas:
    /// ```ignore
    /// EQU <Ident>
    /// EQU <Number>
    /// EQU +<Number>
    /// EQU -<Number>
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: sufixo após `EQU`;
    /// - `fallback_span`: span de fallback quando o valor está ausente.
    ///
    /// Retorno:
    /// - `Ok((value, tail, value_span))`, onde:
    ///   - `value` é `Operand::Ident` ou `Operand::Number`;
    ///   - `tail` são os tokens remanescentes após o valor;
    ///   - `value_span` é o span do token de valor.
    ///
    /// Erros:
    /// - `MissingToken` quando não há valor após `EQU`;
    /// - `InvalidEquDirectiveSyntatic(InvalidValueType)` quando o token de
    ///   valor não é `Ident`, `Number`, `+Number` ou `-Number`.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_equ_value<'a>(
        &mut self,
        line: &'a [Token],
        fallback_span: Span,
    ) -> Result<(Operand, &'a [Token], Span), PreprocessorError> {
        let generic_ident = self.fixed_symbols.generic_ident;

        self.parse_operand(line, fallback_span)
            .map_err(|err| match err.kind {
                super::OperandParseErrorKind::Missing => Self::directive_sintatic_error(
                    DirectiveSyntaxErrorKind::MissingToken {
                        directive: DirectiveKind::Equ,
                        token_missed: TokenKind::Ident(generic_ident),
                    },
                    err.span,
                ),
                super::OperandParseErrorKind::InvalidType => Self::equ_directive_sintatic_error(
                    EquDirectiveSyntaticErrorKind::InvalidValueType,
                    err.span,
                ),
            })
    }

    /// Constrói erro específico da família `EQU`.
    ///
    /// Encapsula `EquDirectiveSyntaticErrorKind` em
    /// `PreprocessorErrorKind::InvalidEquDirectiveSyntatic`, preservando o `span`.
    fn equ_directive_sintatic_error(
        kind: EquDirectiveSyntaticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: InvalidEquDirectiveSyntatic(kind),
            span,
        }
    }
}
