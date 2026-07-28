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
//! - este módulo resolve significado semântico da condição (`NumericLiteral`/`Ident`).

use crate::{
    assembler::errors::{IfDirectiveSemanticErrorKind, PreprocessorError, PreprocessorErrorKind},
    assembler::interner::{Interner, Symbol},
    assembler::language::numeric_literals::NumberParseError,
    assembler::lexer::{Span, Token, TokenKind},
    assembler::preprocessor::{
        LogicalLineIter, Preprocessor,
        ir::{IfDecl, NodeId, Operand},
        types::EquReplacement,
    },
};

impl Preprocessor<'_> {
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
    pub(in crate::assembler::preprocessor) fn execute_if_directive(
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
    /// - `Operand::Number(number)`: parseia como `i16`/`u16` conforme a presença
    ///   de sinal explícito no literal;
    /// - `Operand::Ident(sym)`: procura `sym` na tabela `EQU` e avalia a
    ///   substituição resolvida.
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
                let replacement = EquReplacement::from_numeric_literal(*number);
                let value = replacement.evaluate(interner).map_err(|err| {
                    self.map_number_parse_err_to_if_semantic_error(
                        err,
                        replacement.number_symbol(),
                        *node_id,
                    )
                })?;
                Ok(value != 0)
            }
            Operand::Ident { sym, node_id } => {
                let replacement = self.equs.get(sym).ok_or_else(|| {
                    Self::if_directive_semantic_error(
                        IfDirectiveSemanticErrorKind::UndefinedIdentifier {
                            ident: TokenKind::Ident(*sym),
                        },
                        self.span_of_node(*node_id),
                    )
                })?;

                let value = replacement.evaluate(interner).map_err(|err| {
                    self.map_number_parse_err_to_if_semantic_error(
                        err,
                        replacement.number_symbol(),
                        *node_id,
                    )
                })?;

                Ok(value != 0)
            }
        }
    }

    /// Converte erro técnico de parsing numérico em erro semântico de `IF`.
    fn map_number_parse_err_to_if_semantic_error(
        &self,
        err: NumberParseError,
        value: Symbol,
        node_id: NodeId,
    ) -> PreprocessorError {
        match err {
            NumberParseError::Overflow => Self::if_directive_semantic_error(
                IfDirectiveSemanticErrorKind::ConditionNumberOverflow { value },
                self.span_of_node(node_id),
            ),
            NumberParseError::InvalidNumber => Self::if_directive_semantic_error(
                IfDirectiveSemanticErrorKind::InvalidConditionNumber { value },
                self.span_of_node(node_id),
            ),
        }
    }

    /// Constrói erro semântico da família `IF` com `span` preservado.
    ///
    /// Encapsula o `kind` em
    /// [`crate::assembler::errors::PreprocessorErrorKind::InvalidIfDirectiveSemantic`].
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
