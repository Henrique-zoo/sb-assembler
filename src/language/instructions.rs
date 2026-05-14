use std::collections::HashMap;

use crate::assembler::Word;

pub(crate) struct InstructionSet {
    pub specs: HashMap<Mnemonic, InstructionSpec>,
}

impl InstructionSet {
    pub(crate) fn new() -> Self {
        use Mnemonic::*;
        use OperandArity::*;

        let specs = [
            (Add, InstructionSpec::new(1, 2, One)),
            (Sub, InstructionSpec::new(2, 2, One)),
            (Mult, InstructionSpec::new(3, 2, One)),
            (Div, InstructionSpec::new(4, 2, One)),
            (Jmp, InstructionSpec::new(5, 2, One)),
            (Jmpn, InstructionSpec::new(6, 2, One)),
            (Jmpp, InstructionSpec::new(7, 2, One)),
            (Jmpz, InstructionSpec::new(8, 2, One)),
            (Copy, InstructionSpec::new(9, 3, Two)),
            (Load, InstructionSpec::new(10, 2, One)),
            (Store, InstructionSpec::new(11, 2, One)),
            (Input, InstructionSpec::new(12, 2, One)),
            (Output, InstructionSpec::new(13, 2, One)),
            (Stop, InstructionSpec::new(14, 1, None)),
        ]
        .into_iter()
        .collect();

        Self { specs }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstructionSpec {
    pub opcode: Word,
    pub size: Word,
    pub operands: OperandArity,
}

impl InstructionSpec {
    fn new(opcode: Word, size: Word, operands: OperandArity) -> Self {
        Self {
            opcode,
            size,
            operands,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandArity {
    None,
    One,
    Two,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) enum Mnemonic {
    Add,
    Sub,
    Mult,
    Div,
    Jmp,
    Jmpn,
    Jmpp,
    Jmpz,
    Copy,
    Load,
    Store,
    Input,
    Output,
    Stop,
}
