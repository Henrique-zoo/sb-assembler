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
            (Mul, InstructionSpec::new(3, 2, One)),
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
    Mul,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum Opcode {
    Add = 0x1,
    Sub = 0x2,
    Mul = 0x3,
    Div = 0x4,
    Jmp = 0x5,
    Jmpn = 0x6,
    Jmpp = 0x7,
    Jmpz = 0x8,
    Copy = 0x9,
    Load = 0xA,
    Store = 0xB,
    Input = 0xC,
    Output = 0xD,
    Stop = 0xE,
}

pub(crate) struct InvalidOpcodeError {
    pub opcode: Word,
}

impl TryFrom<Word> for Opcode {
    type Error = InvalidOpcodeError;
    fn try_from(value: Word) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(Opcode::Add),
            0x2 => Ok(Opcode::Sub),
            0x3 => Ok(Opcode::Mul),
            0x4 => Ok(Opcode::Div),
            0x5 => Ok(Opcode::Jmp),
            0x6 => Ok(Opcode::Jmpn),
            0x7 => Ok(Opcode::Jmpp),
            0x8 => Ok(Opcode::Jmpz),
            0x9 => Ok(Opcode::Copy),
            0xA => Ok(Opcode::Load),
            0xB => Ok(Opcode::Store),
            0xC => Ok(Opcode::Input),
            0xD => Ok(Opcode::Output),
            0xE => Ok(Opcode::Stop),
            _ => Err(InvalidOpcodeError { opcode: value }),
        }
    }
}
