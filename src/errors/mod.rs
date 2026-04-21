//! Tipos de erro compartilhados entre os estágios do assembler.
//!
//! Este módulo centraliza os diagnósticos emitidos pelos componentes internos,
//! mantendo uma API única para:
//! - léxico ([`lexer`]);
//! - pré-processamento ([`preprocessor`]).
//!
//! Objetivos principais:
//! - padronizar o formato dos erros;
//! - preservar `Span` para mensagens precisas;
//! - facilitar evolução do pipeline sem espalhar definições de erro.

mod lexer;
mod preprocessor;

/// Reexports de erros léxicos.
pub(crate) use lexer::*;
/// Reexports de erros do pré-processador.
pub(crate) use preprocessor::*;
