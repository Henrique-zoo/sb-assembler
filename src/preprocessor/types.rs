use std::collections::HashMap;

use crate::{errors::PreprocessorError, interner::Symbol, lexer::Token};

pub(crate) type Param = Symbol;

#[derive(Debug, Clone)]
pub(crate) struct MacroHeader {
    pub name: Symbol,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone)]
pub(crate) struct Macro {
    pub header: MacroHeader,
    pub body: Vec<Vec<Token>>,
}

#[derive(Debug, Clone)]
pub(crate) enum State {
    Normal,
    DefiningMacro {
        name: Symbol,
        params: Vec<Symbol>,
        body: Vec<Vec<Token>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Keywords {
    pub macro_kw: Symbol,
    pub endmacro_kw: Symbol,
    pub equ_kw: Symbol,
    pub if_kw: Symbol,
    pub org_kw: Symbol,
}

#[derive(Debug)]
pub(crate) struct Preprocessor {
    pub(super) macros: HashMap<Symbol, Macro>,
    pub(super) equs: HashMap<Symbol, Symbol>,
    pub(super) state: State,
    pub(super) keywords: Keywords,
}

pub(super) struct PreprocessorAcc {
    pub output: Vec<Token>,
    pub current_line: Vec<Token>,
    pub errors: Vec<PreprocessorError>,
}

impl PreprocessorAcc {
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            current_line: Vec::new(),
            errors: Vec::new(),
        }
    }
}
