//! Tipos de erro compartilhados entre os estágios do assembler.
//!
//! Este módulo centraliza os diagnósticos emitidos pelos componentes internos,
//! mantendo uma API única para:
//! - léxico;
//! - pré-processamento;
//! - montagem.
//!
//! Objetivos principais:
//! - padronizar o formato dos erros;
//! - preservar `Span` para mensagens precisas;
//! - facilitar evolução do pipeline sem espalhar definições de erro.

mod assembly;
mod lexer;
mod preprocessor;
/// Reexports de erros de montagem.
pub(crate) use assembly::*;
/// Reexports de erros léxicos.
pub(crate) use lexer::*;
/// Reexports de erros do pré-processador.
pub(crate) use preprocessor::*;
