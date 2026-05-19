//! Internamento de strings para símbolos compactos.
//!
//! Este módulo evita duplicação de lexemas no lexer/preprocessor ao mapear cada
//! string distinta para um identificador numérico (`Symbol`).
//!
//! Benefícios:
//! - comparação rápida por inteiro (`u32`) em vez de `String`;
//! - menor consumo de memória para lexemas repetidos;
//! - mapeamento estável durante a execução do processo.

use std::{collections::HashMap, rc::Rc};

/// Identificador compacto de uma string internada.
pub type Symbol = u32;

/// Resultado de uma consulta `entry` ao interner.
///
/// Segue o padrão "occupied/vacant":
/// - [`Entry::Occupied`] quando a string já possui símbolo;
/// - [`Entry::Vacant`] quando ainda precisa ser inserida.
pub enum Entry<'a> {
    /// Entrada já existente no interner.
    Occupied(Symbol),
    /// Entrada ausente, carregando contexto para inserção.
    Vacant {
        /// Referência mutável ao interner dono.
        interner: &'a mut Interner,
        /// Chave textual pendente de inserção.
        key: String,
    },
}

/// Estrutura de internamento baseada em mapa + vetor.
///
/// Invariantes esperadas:
/// - para toda chave em `map`, `vec[symbol]` contém a string correspondente;
/// - `symbol` é sempre um índice válido no `vec`.
/// - `map` e `vec` compartilham a mesma alocação textual via [`Rc<str>`].
#[derive(Debug)]
pub struct Interner {
    /// Mapeia string -> símbolo.
    map: HashMap<Rc<str>, Symbol>,
    /// Mapeia símbolo (índice) -> string.
    vec: Vec<Rc<str>>,
}

impl Interner {
    /// Cria um interner vazio.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            vec: Vec::new(),
        }
    }

    /// Consulta uma string no interner.
    ///
    /// Retorna:
    /// - [`Entry::Occupied`] se `s` já estiver internada;
    /// - [`Entry::Vacant`] com contexto para inserção posterior.
    ///
    /// O lifetime `'a` garante que a entrada `Vacant` não vive mais que o
    /// `Interner` mutável de origem.
    pub fn entry<'a>(&'a mut self, s: &str) -> Entry<'a> {
        if let Some(&sym) = self.map.get(s) {
            Entry::Occupied(sym)
        } else {
            Entry::Vacant {
                interner: self,
                key: s.to_string(),
            }
        }
    }

    pub fn get_str(&self, sym: Symbol) -> Option<&str> {
        self.vec.get(sym as usize).map(|s| s.as_ref())
    }
}

impl<'a> Entry<'a> {
    /// Obtém o símbolo existente ou insere a chave pendente.
    ///
    /// Em `Occupied`, apenas retorna o símbolo já associado.
    /// Em `Vacant`, insere a chave, cria novo símbolo e retorna esse símbolo.
    pub fn or_insert(self) -> Symbol {
        match self {
            Entry::Occupied(sym) => sym,
            Entry::Vacant { interner, key } => {
                let sym = interner.vec.len() as Symbol;
                let key_rc: Rc<str> = Rc::from(key);

                interner.vec.push(Rc::clone(&key_rc));
                interner.map.insert(key_rc, sym);

                sym
            }
        }
    }
}
