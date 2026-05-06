//! Execução semântica da diretiva `EQU`.
//!
//! Este módulo aplica o significado de `EQU` após o parsing sintático:
//! - resolve o operando de valor (`Ident` ou `Number`);
//! - registra `alias -> valor` na tabela de símbolos `EQU`.
//!
//! Observação arquitetural:
//! - o parser (`parser/equ_parser.rs`) valida a forma da linha;
//! - este módulo resolve referência simbólica e atualiza estado.

use crate::{
    errors::{EquDirectiveSemanticErrorKind, PreprocessorError, PreprocessorErrorKind},
    interner::{Interner, Symbol},
    lexer::Span,
    preprocessor::{
        NumberParseError, Preprocessor,
        ir::{EquDecl, NodeId, Number, Operand},
        parse_signed_number, parse_unsigned_number,
        types::EquValue,
    },
};

impl Preprocessor {
    /// Executa semanticamente uma diretiva `EQU` já parseada.
    ///
    /// Forma canônica que chega aqui (já validada no parser):
    /// ```ignore
    /// <Alias> EQU <Number|Ident>
    /// ```
    ///
    /// Fluxo:
    /// 1. resolve o operando com [`Self::resolve_operand`];
    /// 2. grava o par `alias -> valor` em `self.equs`.
    ///
    /// Retorno:
    /// - `Ok(())` quando a resolução e a escrita na tabela são bem-sucedidas.
    ///
    /// Erros:
    /// - propaga erro semântico de [`Self::resolve_operand`] quando o valor
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
        let operand_val = self.resolve_operand(&equ_decl.value, interner)?;
        self.equs.insert(equ_decl.alias, operand_val);
        Ok(())
    }

    /// Resolve o valor simbólico de um operando de `EQU`.
    ///
    /// Regras:
    /// - `Operand::Number`: parseia o literal para `i16`/`u16` e converte para
    ///   [`EquValue`];
    /// - `Operand::Ident { sym, .. }`: busca `sym` em `self.equs`.
    ///
    /// Retorno:
    /// - `Ok(EquValue)` quando o operando pode ser resolvido.
    ///
    /// Erros:
    /// - `InvalidEquDirectiveSemantic(InvalidValue)` quando `Operand::Ident`
    ///   referencia alias inexistente;
    /// - `InvalidEquDirectiveSemantic(InvalidValueNumber)` quando o literal de
    ///   `Operand::Number` não é parseável;
    /// - `InvalidEquDirectiveSemantic(ValueNumberOverflow)` quando o literal de
    ///   `Operand::Number` estoura `u16`/`i16`.
    fn resolve_operand(
        &self,
        operand: &Operand,
        interner: &Interner,
    ) -> Result<EquValue, PreprocessorError> {
        match operand {
            Operand::Number { number, node_id } => match number {
                Number::Signed { .. } => parse_signed_number(number, interner)
                    .map(EquValue::Signed)
                    .map_err(|err| {
                        self.map_number_parse_err_to_equ_semantic_error(err, number.sym(), *node_id)
                    }),
                Number::Unsigned { .. } => parse_unsigned_number(number, interner)
                    .map(EquValue::Unsigned)
                    .map_err(|err| {
                        self.map_number_parse_err_to_equ_semantic_error(err, number.sym(), *node_id)
                    }),
            },
            Operand::Ident { sym, node_id } => {
                let &number = self.equs.get(sym).ok_or_else(|| {
                    Self::equ_directive_semantic_error(
                        EquDirectiveSemanticErrorKind::InvalidValue,
                        self.span_of_node(*node_id),
                    )
                })?;

                Ok(number)
            }
        }
    }

    /// Converte erro técnico de parsing numérico em erro semântico de `EQU`.
    ///
    /// Esse mapeamento mantém o pré-processador como fronteira de diagnóstico:
    /// o parser numérico interno retorna [`NumberParseError`], e o estágio de
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
}
