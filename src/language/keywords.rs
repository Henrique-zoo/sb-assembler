use std::collections::HashSet;

use crate::interner::{Interner, Symbol};

#[derive(Debug)]
pub(crate) struct KeywordTable {
    directives: HashSet<Symbol>,
    instructions: HashSet<Symbol>,
}

impl KeywordTable {
    pub(crate) fn new(interner: &mut Interner) -> Self {
        let directives = ["SECTION", "TEXT", "DATA", "MACRO", "ENDMACRO", "EQU", "IF"]
            .into_iter()
            .map(|kw| interner.entry(kw).or_insert())
            .collect();

        let instructions = [
            "ADD", "SUB", "MULT", "DIV", "JMP", "JMPN", "JMPP", "JMPZ", "COPY", "LOAD", "STORE",
            "INPUT", "OUTPUT", "STOP",
        ]
        .into_iter()
        .map(|kw| interner.entry(kw).or_insert())
        .collect();

        Self {
            directives,
            instructions,
        }
    }

    pub(crate) fn is_directive(&self, sym: Symbol) -> bool {
        self.directives.contains(&sym)
    }

    pub(crate) fn is_instruction(&self, sym: Symbol) -> bool {
        self.instructions.contains(&sym)
    }

    pub(crate) fn is_reserved(&self, sym: Symbol) -> bool {
        self.is_directive(sym) || self.is_instruction(sym)
    }
}
