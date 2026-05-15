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
    pub artifacts: AssemblyArtifacts,
}

impl<'a> OnePassAssembler<'a> {
    /// Cria um montador de uma passagem com estado semântico vazio.
    ///
    /// O montador guarda referências para as tabelas compartilhadas da
    /// linguagem e inicializa o estado mutável usado durante a passagem:
    /// tabela de símbolos, contador de posição, lista de diagnósticos e
    /// artefatos de saída.
    ///
    /// # Parâmetros
    /// - `interner`: tabela usada para converter símbolos internados em
    ///   lexemas durante validação numérica;
    /// - `isa`: especificação de opcodes e tamanhos das instruções.
    ///
    /// # Retorno
    /// - [`OnePassAssembler`] pronto para consumir um
    ///   [`crate::parser::types::ParsedProgram`].
    pub(crate) fn new(interner: &'a Interner, isa: &'a InstructionSet) -> Self {
        Self {
            interner,
            isa,
            symbols: SymbolTable::new(),
            location_counter: 0,
            errors: Vec::new(),
            artifacts: AssemblyArtifacts {
                object: ObjectProgram { words: Vec::new() },
                pending: PendingProgram { words: Vec::new() },
            },
        }
    }
}

pub(crate) struct SymbolTable {
    pub entries: HashMap<Symbol, SymbolEntry>,
}

impl SymbolTable {
    /// Cria uma tabela de símbolos vazia.
    ///
    /// # Retorno
    /// - [`SymbolTable`] sem definições nem usos pendentes.
    fn new() -> Self {
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
