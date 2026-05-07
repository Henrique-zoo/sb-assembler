use std::collections::HashMap;

use crate::assembler::Word;

pub(crate) struct InstructionSet {
    specs: HashMap<Mnemonic, InstructionSpec>,
}

pub(crate) struct InstructionSpec {
    pub opcode: Word,
    pub size: Word,
    pub operands: OperandArity,
}

pub(crate) enum OperandArity {
    None,
    One,
    Two,
}

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