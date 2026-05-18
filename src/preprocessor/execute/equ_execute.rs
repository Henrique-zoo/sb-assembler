//! Execução semântica da diretiva `EQU`.
//!
//! Este módulo aplica o significado de `EQU` após o parsing sintático:
//! - resolve o operando de valor (`Ident` ou `Number`);
//! - registra `alias -> substituição` na tabela de símbolos `EQU`.
//!
//! Observação arquitetural:
//! - o parser (`parser/equ_parser.rs`) valida a forma da linha;
//! - este módulo resolve referência simbólica e atualiza estado.

use crate::{
    errors::{EquDirectiveSemanticErrorKind, PreprocessorError, PreprocessorErrorKind},
    interner::{Interner, Symbol},
    language::numeric_literals::{self, NumberParseError, NumericLiteral},
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        Preprocessor,
        ir::{EquDecl, NodeId, Operand},
        types::EquReplacement,
    },
};

impl Preprocessor<'_> {
    /// Executa semanticamente uma diretiva `EQU` já parseada.
    ///
    /// Forma canônica que chega aqui (já validada no parser):
    /// ```ignore
    /// <Alias> EQU <Number|Ident>
    /// ```
    ///
    /// Fluxo:
    /// 1. resolve o operando com [`Self::resolve_equ_replacement`];
    /// 2. grava o par `alias -> substituição` em `self.equs`.
    ///
    /// Retorno:
    /// - `Ok(())` quando a resolução e a escrita na tabela são bem-sucedidas.
    ///
    /// Erros:
    /// - propaga erro semântico de [`Self::resolve_equ_replacement`] quando o valor
    ///   é um identificador sem definição `EQU` prévia.
    ///
    /// Efeito colateral:
    /// - muta `self.equs`;
    /// - se `alias` já existia, o valor anterior é sobrescrito.
    pub(in crate::preprocessor) fn execute_equ_directive(
        &mut self,
        equ_decl: EquDecl,
        interner: &Interner,
    ) -> Result<(), PreprocessorError> {
        let replacement = self.resolve_equ_replacement(&equ_decl.value, interner)?;
        self.equs.insert(equ_decl.alias, replacement);
        Ok(())
    }

    /// Resolve a substituição associada a um operando de `EQU`.
    ///
    /// Regras:
    /// - `Operand::Number`: valida o literal com o parser numérico da linguagem
    ///   e preserva sua forma reemitível;
    /// - `Operand::Ident { sym, .. }`: busca `sym` em `self.equs`.
    ///
    /// Retorno:
    /// - `Ok(EquReplacement)` quando o operando pode ser resolvido.
    ///
    /// Erros:
    /// - `InvalidEquDirectiveSemantic(UndefinedSymbol)` quando `Operand::Ident`
    ///   referencia alias inexistente;
    /// - `InvalidEquDirectiveSemantic(InvalidValueNumber)` quando o literal de
    ///   `Operand::Number` não é parseável;
    /// - `InvalidEquDirectiveSemantic(ValueNumberOverflow)` quando o literal de
    ///   `Operand::Number` estoura `u16`/`i16`.
    fn resolve_equ_replacement(
        &self,
        operand: &Operand,
        interner: &Interner,
    ) -> Result<EquReplacement, PreprocessorError> {
        match operand {
            Operand::Number {
                number:
                    NumericLiteral {
                        sign: Some(sign),
                        symbol,
                    },
                node_id,
            } => numeric_literals::parse_signed_symbol(*sign, *symbol, interner)
                .map(|_| EquReplacement::Signed {
                    sign: *sign,
                    number: *symbol,
                })
                .map_err(|err| {
                    self.map_number_parse_err_to_equ_semantic_error(err, *symbol, *node_id)
                }),
            Operand::Number {
                number: NumericLiteral { sign: None, symbol },
                node_id,
            } => numeric_literals::parse_unsigned_symbol(*symbol, interner)
                .map(|_| EquReplacement::Unsigned { number: *symbol })
                .map_err(|err| {
                    self.map_number_parse_err_to_equ_semantic_error(err, *symbol, *node_id)
                }),
            Operand::Ident { sym, node_id } => {
                let &replacement = self.equs.get(sym).ok_or_else(|| {
                    Self::equ_directive_semantic_error(
                        EquDirectiveSemanticErrorKind::UndefinedSymbol { symbol: *sym },
                        self.span_of_node(*node_id),
                    )
                })?;

                Ok(replacement)
            }
        }
    }

    /// Converte erro técnico de parsing numérico em erro semântico de `EQU`.
    ///
    /// Esse mapeamento mantém o pré-processador como fronteira de diagnóstico:
    /// o módulo numérico da linguagem retorna [`NumberParseError`], e o estágio de
    /// execução converte para [`PreprocessorError`], com `span` do operando.
    fn map_number_parse_err_to_equ_semantic_error(
        &self,
        err: NumberParseError,
        value: Symbol,
        node_id: NodeId,
    ) -> PreprocessorError {
        match err {
            NumberParseError::InvalidNumber => Self::equ_directive_semantic_error(
                EquDirectiveSemanticErrorKind::InvalidValueNumber { value },
                self.span_of_node(node_id),
            ),
            NumberParseError::Overflow => Self::equ_directive_semantic_error(
                EquDirectiveSemanticErrorKind::ValueNumberOverflow { value },
                self.span_of_node(node_id),
            ),
        }
    }

    /// Constrói erro semântico da família `EQU` com `span` preservado.
    ///
    /// Encapsula o `kind` em
    /// [`crate::errors::PreprocessorErrorKind::InvalidEquDirectiveSemantic`].
    fn equ_directive_semantic_error(
        kind: EquDirectiveSemanticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidEquDirectiveSemantic(kind),
            span,
        }
    }

    /// Substitui alias `EQU` usado como operando de diretiva `DATA`.
    ///
    /// Forma esperada:
    /// ```ignore
    /// <label>: CONST <alias>
    /// <label>: SPACE <alias>
    /// ```
    ///
    /// A substituição usa o span do ponto de uso, para que diagnósticos
    /// posteriores apontem para a linha que consumiu o alias, não para a
    /// declaração original do `EQU`.
    ///
    /// Retorno:
    /// - `Ok(())` quando a linha não exige substituição ou quando o alias foi
    ///   materializado com sucesso;
    /// - `Err(InvalidEquDirectiveSemantic(UndefinedSymbol))` quando o operando
    ///   parece um alias `EQU`, mas ainda não existe em `self.equs`.
    pub(in crate::preprocessor) fn replace_data_equ_use(
        &self,
        line: &mut Vec<Token>,
    ) -> Result<(), PreprocessorError> {
        let Some(Token {
            kind: TokenKind::Ident(sym),
            span,
        }) = line.get(3).copied()
        else {
            return Ok(());
        };

        let replacement = self.equs.get(&sym).ok_or_else(|| {
            Self::equ_directive_semantic_error(
                EquDirectiveSemanticErrorKind::UndefinedSymbol { symbol: sym },
                span,
            )
        })?;

        line.splice(3..4, replacement.to_tokens_with_span(span));
        Ok(())
    }
}
