//! Erros gerados durante a análise léxica.
//!
//! O lexer converte texto em tokens e, quando encontra lexemas inválidos,
//! produz diagnósticos com localização exata via [`Span`].
//!
//! Este módulo define:
//! - categorias de falha léxica ([`LexerErrorKind`]);
//! - payload completo do diagnóstico ([`LexerError`]).

use crate::lexer::Span;

/// Categoria de erro produzido pelo lexer.
#[derive(Debug)]
pub enum LexerErrorKind {
    /// Caractere isolado que não pertence ao alfabeto léxico esperado.
    InvalidChar(char),
    /// Sequência que aparenta ser identificador, mas quebra as regras da
    /// linguagem.
    InvalidIdentifier(String),
    /// Sequência que aparenta ser número, mas não representa literal válido.
    InvalidNumber(String),
}

/// Diagnóstico léxico completo.
///
/// Combina a categoria da falha com o `Span` que permite apontar a origem do
/// problema no fonte.
#[derive(Debug)]
pub struct LexerError {
    /// Tipo específico da falha léxica.
    pub kind: LexerErrorKind,
    /// Região do código-fonte associada ao erro.
    pub span: Span,
}
