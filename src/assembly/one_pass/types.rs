use std::collections::HashMap;

use crate::{
    assembler::Word,
    errors::AssemblyError,
    interner::{Interner, Symbol},
    language::instructions::InstructionSet,
    lexer::Span,
};

pub(crate) struct OnePassAssembler<'a> {
    pub interner: &'a Interner,
    pub isa: &'a InstructionSet,

    pub symbols: SymbolTable,
    pub location_counter: usize,
    pub errors: Vec<AssemblyError>,
}

pub(crate) struct SymbolTable {
    pub entries: HashMap<Symbol, SymbolEntry>,
}

pub(crate) enum SymbolEntry {
    Defined { address: Word, defined_at: Span },
    Pending { uses: Vec<PatchSite> },
}

pub(crate) struct PatchSite {
    pub output_index: usize,
    pub span: Span,
}

/*
pub(crate) struct DefinitionEntry {
    pub name: String,
    pub address: Word,
}

pub(crate) struct UseEntry {
    pub name: String,
    pub positions: Vec<Word>,
}
*/

pub(crate) struct AssemblyArtifacts {
    pub object: ObjectProgram,
    pub pending: PendingProgram,
}

pub(crate) struct PendingProgram {
    pub words: Vec<Word>,
}

pub(crate) struct ObjectProgram {
    pub words: Vec<Word>,
    // pub use_table: Vec<UseEntry>,
    // pub definition_table: Vec<DefinitionEntry>,
}

pub(crate) type Offset<T> = T;
