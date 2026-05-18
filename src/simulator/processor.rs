//! Estado interno da máquina simulada.
//!
//! Este submódulo define o processador que recebe um [`ObjectProgram`], copia
//! suas palavras para a memória e mantém os registradores necessários para a
//! execução. A decodificação e o efeito das instruções ficam no módulo pai,
//! porque fazem parte da orquestração do simulador.

use crate::{
    assembler::{SignedWord, Word},
    assembly::one_pass::ObjectProgram,
    errors::SimulationError,
};

/// Quantidade de palavras endereçáveis pela máquina simulada.
const MEMORY_SIZE: usize = 65_536;

/// Processador da máquina alvo.
///
/// O processador guarda apenas estado de execução. Ele não sabe ler arquivos
/// `.obj` nem conhece o pipeline de montagem; seu contrato começa em
/// [`Processor::load`], quando um [`ObjectProgram`] já resolvido é copiado para
/// a memória.
pub(crate) struct Processor {
    /// Acumulador aritmético.
    ///
    /// É assinado para que instruções condicionais como `JMPN` e `JMPP`
    /// observem o sinal do último valor carregado ou calculado.
    pub acc: SignedWord,
    /// Contador de programa.
    ///
    /// Aponta para a próxima palavra que deve ser interpretada como opcode.
    pub pc: usize,
    /// Memória da máquina simulada.
    ///
    /// Cada posição armazena uma [`Word`]. Código e dados compartilham o mesmo
    /// espaço; o papel de uma palavra depende de como ela é acessada durante a
    /// execução.
    pub memory: Vec<Word>,
}

impl Processor {
    /// Cria um processador vazio.
    ///
    /// # Retorno
    /// - [`Processor`] com acumulador zerado, `pc = 0` e memória preenchida com
    ///   zero.
    pub(in crate::simulator) fn new() -> Self {
        Self {
            acc: 0,
            pc: 0,
            memory: vec![0; MEMORY_SIZE],
        }
    }

    /// Carrega um programa objeto na memória.
    ///
    /// O carregamento reinicia o estado de execução: acumulador zerado,
    /// contador de programa em `0` e memória preenchida com zero antes da cópia
    /// das palavras do programa.
    ///
    /// # Parâmetros
    /// - `program`: programa objeto emitido pela montagem, contendo `TEXT`
    ///   seguido de `DATA`.
    ///
    /// # Retorno
    /// - `Ok(())` quando todas as palavras foram carregadas;
    /// - `Err(SimulationError)` quando o programa excede a memória disponível.
    ///
    /// # Erros
    /// Retorna [`SimulationError::ExceededMemorySize`] se a quantidade de
    /// palavras do programa for maior que a memória simulada.
    pub(in crate::simulator) fn load(
        &mut self,
        program: ObjectProgram,
    ) -> Result<(), SimulationError> {
        if program.words.len() > MEMORY_SIZE {
            return Err(SimulationError::ExceededMemorySize);
        }

        self.acc = 0;
        self.pc = 0;
        self.memory.fill(0);

        for (address, word) in program.words.into_iter().enumerate() {
            self.memory[address] = word;
        }

        Ok(())
    }
}
