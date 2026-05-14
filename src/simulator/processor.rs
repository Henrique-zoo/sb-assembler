use crate::assembler::Word;

pub(crate) struct Processor {
    pub acc: Word,
    pub memory: [Word; 65_536],
}
