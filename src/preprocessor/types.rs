use crate::interner::Symbol;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub line: u32,
    pub column: u32,
    pub pos: usize,
    pub len: usize,
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(Symbol),      // COPY, LOAD, A, TEMP
    Number(i32),
    Comma,
    Colon,
    Plus,
    Minus,
    NewLine,
    EOF,
    Placeholder(usize), // #0, #1

}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub token_kind: TokenKind,
    pub span: Span,
    pub origin: Option<Span>,
}