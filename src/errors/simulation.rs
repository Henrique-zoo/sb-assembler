use crate::assembler::Word;

/// Erros de simulação
#[derive(Debug, Clone)]
pub enum SimulationError {
    /// O arquivo `.obj` não pôde ser lido.
    ObjectFileRead { path: String, reason: String },
    /// O arquivo `.obj` tem uma quantidade incompleta de bytes.
    MisalignedObjectFile { bytes: usize },
    /// O programa excede o tamanho da memória
    ExceededMemorySize,
    /// O contador de programa saiu da memória disponível.
    ProgramCounterOutOfBounds { pc: usize },
    /// Opcode inválido
    InvalidOpcode { opcode: Word, pc: usize },
    /// Divisão por zero durante a execução.
    DivisionByZero { pc: usize },
    /// Entrada de tipo inválido
    InvalidInputType,
}
