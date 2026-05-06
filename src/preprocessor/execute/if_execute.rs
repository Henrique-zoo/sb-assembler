//! Execução semântica da diretiva `IF`.
//!
//! Este módulo aplica o comportamento de include condicional após o parsing
//! sintático de `IF`:
//! - condição verdadeira: consome a próxima linha lógica e devolve seu
//!   conteúdo para emissão;
//! - condição falsa: também consome a próxima linha lógica, mas descarta seu
//!   conteúdo.
//!
//! Observação arquitetural:
//! - o parser (`parser/if_parser.rs`) valida forma;
//! - este módulo resolve significado semântico da condição (`Number`/`Ident`).

use crate::{
    errors::{IfDirectiveSemanticErrorKind, PreprocessorError, PreprocessorErrorKind},
    interner::Interner,
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        LogicalLineIter, NumberParseError, Preprocessor,
        ir::{IfDecl, Number, Operand},
        parse_signed_number, parse_unsigned_number,
        types::EquValue,
    },
};

impl Preprocessor {
    /// Executa semanticamente uma diretiva `IF` já parseada.
    ///
    /// Forma canônica que chega aqui (já validada no parser):
    /// ```ignore
    /// IF <Number|Ident>
    /// ```
    ///
    /// Fluxo:
    /// 1. resolve a condição com [`Self::resolve_condition`];
    /// 2. consome a próxima linha lógica de `lines`;
    /// 3. se a condição for verdadeira, retorna o `content` da linha
    ///    consumida;
    /// 4. se a condição for falsa, descarta a linha consumida e retorna
    ///    `None`.
    ///
    /// Retorno:
    /// - `Ok(Some(tokens))` quando a condição é verdadeira e há linha
    ///   subsequente para incluir;
    /// - `Ok(None)` quando a condição é falsa.
    ///
    /// Erros:
    /// - propaga erros semânticos de resolução da condição;
    /// - `InvalidIfDirectiveSemantic(MissingNextLine)` quando não existe linha
    ///   seguinte consumível após o `IF`.
    ///
    /// Efeito colateral:
    /// - avança o iterador `lines` em uma posição para consumir a linha
    ///   imediatamente seguinte ao `IF`, independentemente da condição.
    pub(in crate::preprocessor) fn execute_if_directive(
        &self,
        if_decl: IfDecl,
        lines: &mut LogicalLineIter,
        interner: &Interner,
    ) -> Result<Option<Vec<Token>>, PreprocessorError> {
        let mut output = None;

        let next_line = lines.next().ok_or_else(|| {
            Self::if_directive_semantic_error(
                IfDirectiveSemanticErrorKind::MissingNextLine,
                self.span_of_node(if_decl.node_id),
            )
        })?;

        if self.resolve_condition(&if_decl.cond, interner)? {
            output = Some(next_line.content);
        }

        Ok(output)
    }

    /// Resolve o valor booleano da condição de `IF`.
    ///
    /// Regras:
    /// - `Operand::Number(number)`: parseia como `i16`/`u16` conforme a variante
    ///   (`Signed`/`Unsigned`) de `number`;
    /// - `Operand::Ident(sym)`: procura `sym` na tabela `EQU` e avalia o valor
    ///   resolvido (`EquValue`).
    ///
    /// Convenção booleana:
    /// - `0` => `false`;
    /// - qualquer valor não-zero => `true`.
    ///
    /// Mapeamento de erros:
    /// - número literal inválido => `InvalidConditionNumber`;
    /// - overflow de número literal => `ConditionNumberOverflow`;
    /// - identificador sem entrada `EQU` => `UndefinedIdentifier`.
    ///
    /// Efeito colateral:
    /// - nenhum. A função apenas consulta estado e converte condição.
    fn resolve_condition(
        &self,
        condition: &Operand,
        interner: &Interner,
    ) -> Result<bool, PreprocessorError> {
        match condition {
            Operand::Number { number, node_id } => {
                let value = match number {
                    Number::Signed { .. } => {
                        parse_signed_number(number, interner).map(|v| v as i32)
                    }
                    Number::Unsigned { .. } => {
                        parse_unsigned_number(number, interner).map(|v| v as i32)
                    }
                }
                .map_err(|err| match err {
                    NumberParseError::Overflow => Self::if_directive_semantic_error(
                        IfDirectiveSemanticErrorKind::ConditionNumberOverflow {
                            value: number.sym(),
                        },
                        self.span_of_node(*node_id),
                    ),
                    NumberParseError::InvalidNumber => Self::if_directive_semantic_error(
                        IfDirectiveSemanticErrorKind::InvalidConditionNumber {
                            value: number.sym(),
                        },
                        self.span_of_node(*node_id),
                    ),
                })?;
                Ok(value != 0)
            }
            Operand::Ident { sym, node_id } => {
                let equ_value = self.equs.get(sym).ok_or_else(|| {
                    Self::if_directive_semantic_error(
                        IfDirectiveSemanticErrorKind::UndefinedIdentifier {
                            ident: TokenKind::Ident(*sym),
                        },
                        self.span_of_node(*node_id),
                    )
                })?;

                let value = match equ_value {
                    EquValue::Signed(v) => *v as i32,
                    EquValue::Unsigned(v) => *v as i32,
                };

                Ok(value != 0)
            }
        }
    }

    /// Constrói erro semântico da família `IF` com `span` preservado.
    ///
    /// Encapsula o `kind` em
    /// [`crate::errors::PreprocessorErrorKind::InvalidIfDirectiveSemantic`].
    fn if_directive_semantic_error(
        kind: IfDirectiveSemanticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidIfDirectiveSemantic(kind),
            span,
        }
    }
}
