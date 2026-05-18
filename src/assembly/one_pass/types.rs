use std::collections::HashMap;

use crate::{assembler::Word, interner::Symbol, lexer::Span};

pub(crate) struct SymbolTable {
    pub entries: HashMap<Symbol, SymbolEntry>,
}

impl SymbolTable {
    /// Cria uma tabela de símbolos vazia.
    ///
    /// # Retorno
    /// - [`SymbolTable`] sem definições nem usos pendentes.
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

pub(crate) enum SymbolEntry {
    Defined {
        address: Word,
        defined_at: Span,
    },
    Pending {
        uses: Vec<PatchSite>,
        list_head: Option<usize>,
    },
}

pub(crate) struct PatchSite {
    pub output_index: usize,
    pub offset: Offset,
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
    /// Código de máquina final do `.obj`, com todas as referências resolvidas.
    pub object: ObjectProgram,
    /// Código de uma passagem do `.pen`, com listas de pendência embutidas.
    pub pending: PendingProgram,
}

pub(crate) struct PendingProgram {
    /// Palavras emitidas pelo algoritmo de passagem única antes dos remendos.
    pub words: Vec<Word>,
}

pub(crate) struct ObjectProgram {
    /// Palavras finais do programa, incluindo `TEXT` seguido de `DATA`.
    pub words: Vec<Word>,
    // pub use_table: Vec<UseEntry>,
    // pub definition_table: Vec<DefinitionEntry>,
}

pub(crate) type Offset = i32;
