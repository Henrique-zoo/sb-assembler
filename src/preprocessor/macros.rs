use crate::{interner::Symbol, preprocessor::types::Token};

pub(super) struct Macro {
    pub name: Symbol,
    pub params: Vec<Symbol>,
    pub body: Vec<Vec<Token>>,
}