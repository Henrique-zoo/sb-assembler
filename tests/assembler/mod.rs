#![allow(dead_code, unused_imports)]

pub type SignedWord = i16;
pub type Word = u16;

#[path = "../../src/assembler/assembly/mod.rs"]
pub(crate) mod assembly;
#[path = "../../src/assembler/errors/mod.rs"]
pub(crate) mod errors;
#[path = "../../src/assembler/interner/mod.rs"]
pub(crate) mod interner;
#[path = "../../src/assembler/language/mod.rs"]
pub(crate) mod language;
#[path = "../../src/assembler/lexer/mod.rs"]
pub(crate) mod lexer;
#[path = "../../src/assembler/parser/mod.rs"]
pub(crate) mod parser;
#[path = "../../src/assembler/preprocessor/mod.rs"]
pub(crate) mod preprocessor;

pub(crate) fn lex(
    source: &str,
    interner: &mut interner::Interner,
) -> Result<Vec<lexer::Token>, errors::LexerError> {
    let lexer = lexer::Lexer::new(source, interner);
    lexer.collect()
}
