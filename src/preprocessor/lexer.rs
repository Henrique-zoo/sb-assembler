use crate::interner::Interner;

pub(super) struct Lexer<'input, 'interner> {
    input: &'input str,
    interner: &'interner mut Interner,
}

impl<'input, 'interner> Lexer<'input, 'interner> {
    pub fn new(input: &'input str, interner: &'interner mut Interner) -> Self {
        Self { input, interner }
    }
}