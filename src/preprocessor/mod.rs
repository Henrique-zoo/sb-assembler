use std::collections::HashMap;

use crate::{interner::{Interner, Symbol}, preprocessor::{lexer::Lexer, macros::Macro, types::Token}};

mod lexer;
mod macros;
mod types;

/*
O Preprocessor carrega:
    ▸ macros → carrega a relação dos nomes das macros com a suas definições
        ▫ cada string tem um symbol associado em interner. Aqui utiliza-se esses symbols associados aos nomes das macros para melhorar o desempenho
    ▸ equs → carrega a relação dos nomes dos equs com com os valores

Definições:
    ▸ MACROS: Associam a uma label uma sequência de enunciados. São como subrotinas, mas, ao invés da execução pular para a sua linha em tempo de execução, as suas linhas são inseridas, em tempo de compilação, na posição da chamada
        ↪ Utilização:
        SWAP: MACRO &A, &B, &T
              COPY &A, &T
              COPY &B, &A
              COPY &T, &B
              ENDMACRO
    ▸ EQUS: Associam a uma label um valor. No preprocessamento, substituímos as labels no decorrer do código pelo valor associado
        ↪ Utilização:
        TAM: EQU 10
*/
pub(crate) struct Preprocessor {
    macros: HashMap<Symbol, Macro>,
    equs: HashMap<Symbol, i32>,
    state: State,
}

pub(crate) enum State {
    Normal,
    DefiningMacro {
        name: Symbol,
        params: Vec<Symbol>,
        body: Vec<Vec<Token>>,
    }
}

impl Preprocessor {
    pub(crate) fn process(
        &mut self,
        interner: &mut Interner,
        input: &str,
    ) -> Vec<Token> {
        let lexer = Lexer::new(input, interner);


        vec![]
    }
}