use std::collections::HashMap;

pub type Symbol = u32;

pub enum Entry<'a> {
    Occupied(Symbol),
    Vacant {
        interner: &'a mut Interner,
        key: String,
    }
}
pub struct Interner {
    map: HashMap<String, Symbol>,
    vec: Vec<String>,
}

impl Interner {
    // O 'a é um lifetime genérico. Aqui, é usado para definir que o retorno da função, `Entry<>`, não pode viver mais do que self (Interner)
    // A notação de função parametrizada "fn foo<>" não é por acaso. De fato, ela não é parametrizada no sentido mais comum: polimorfismo paramétrico, mas o lifetime 'a é uma "variável de lifetime" cujo valor real depende de quem chama a função, isto é, a variável de lifetime é avaliada para o lifetime de self
    pub fn entry<'a>(&'a mut self, s: &str) -> Entry<'a> {
        if let Some(&sym) = self.map.get(s) {
            Entry::Occupied(sym)
        } else {
            Entry::Vacant {
                interner: self,
                key: s.to_string()
            }
        }
    }
}

impl<'a> Entry<'a> {
    pub fn or_insert(self) -> Symbol {
        match self {
            Entry::Occupied(sym) => sym,
            Entry::Vacant { interner, key } => {
                let sym = interner.vec.len() as Symbol;

                interner.vec.push(key.clone());
                interner.map.insert(key, sym);

                sym
            }
        }
    }
}