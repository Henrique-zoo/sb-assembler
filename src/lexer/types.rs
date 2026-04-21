//! Tipos fundamentais produzidos pelo lexer.
//!
//! O lexer converte texto-fonte em uma sequência de [`Token`], e este módulo
//! define os blocos de dados dessa sequência.
//!
//! Convenções importantes usadas por esses tipos:
//! - posições são baseadas em byte quando se fala de `pos`/`len`;
//! - linha e coluna são 1-based;
//! - lexemas textuais (`Ident`, `Number`) são representados por [`Symbol`]
//!   internado, e não por `String`.

use crate::interner::Symbol;

/// Faixa de origem de um token no arquivo-fonte.
///
/// `Span` aponta para o primeiro byte do token e guarda seu comprimento em
/// bytes. Isso permite mapear tokens de volta ao texto original para mensagens
/// de erro, diagnósticos e tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    /// Offset absoluto (em bytes) do início do token.
    pub pos: usize,
    /// Linha de início do token (1-based).
    pub line: u32,
    /// Coluna de início do token (1-based).
    pub column: u32,
    /// Comprimento do token em bytes.
    ///
    /// Para `Eof`, o valor esperado é `0`.
    pub len: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            pos: 0,
            line: 1,
            column: 1,
            len: 0,
        }
    }
}

/// Categoria léxica de um [`Token`].
///
/// Variantes que carregam [`Symbol`] referenciam texto internado no
/// `Interner`, evitando cópias repetidas de lexemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// Identificador internado (`[A-Za-z_][A-Za-z0-9_]*`).
    Ident(Symbol),
    /// Literal numérico internado (decimal, hexadecimal ou binário).
    Number(Symbol),

    /// `&`, usado como prefixo de parâmetro de macro.
    Ampersand,
    /// `,` (separador de itens/parâmetros).
    Comma,
    /// `:` (separador de rótulo ou marcador sintático).
    Colon,
    /// `+`.
    Plus,
    /// `-`.
    Minus,

    /// Quebra de linha (`\n`) materializada como token.
    NewLine,
    /// Marcador de fim de arquivo, emitido uma única vez.
    Eof,
}

/// Unidade léxica emitida pelo lexer.
///
/// Um token sempre combina:
/// - classe léxica (`kind`);
/// - localização no fonte (`span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    /// Classe léxica do token.
    pub kind: TokenKind,
    /// Região do fonte correspondente ao token.
    pub span: Span,
}

impl Token {
    /// Constrói um token a partir de sua classe e de seu span.
    ///
    /// Esse construtor não aplica validações adicionais: assume que `kind` e
    /// `span` já foram calculados corretamente pelo lexer.
    pub(crate) fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
