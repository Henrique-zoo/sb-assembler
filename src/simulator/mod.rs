//! Simulação do programa objeto.
//!
//! Este módulo é o ponto de entrada do estágio que executa o `.obj` emitido
//! pela montagem. Ele não conhece fonte assembly, seções, rótulos, macros ou
//! diretivas montáveis: sua entrada já é um `ObjectProgram` resolvido, ou um
//! arquivo `.obj` binário que pode ser convertido nessa estrutura.
//!
//! ## Papel no pipeline
//!
//! A simulação começa depois que o pipeline de montagem já transformou o
//! programa em palavras de máquina. O contrato esperado é:
//! 1. o arquivo `.obj` contém apenas palavras de 16 bits;
//! 2. as referências simbólicas já foram resolvidas para endereços numéricos;
//! 3. o vetor de palavras representa `TEXT` seguido de `DATA`;
//! 4. a execução inicia no endereço `0` e termina ao encontrar `STOP`.
//!
//! O simulador carrega todas as palavras na memória e interpreta apenas os
//! endereços alcançados pelo contador de programa. A região de dados não é
//! marcada separadamente no arquivo: ela só será executada se o programa fizer
//! o fluxo de controle chegar até lá.
//!
//! ## Formato do `.obj`
//!
//! O arquivo objeto é tratado como binário, não como texto. Cada [`Word`] ocupa
//! dois bytes e é lida em little-endian, isto é, o byte menos significativo vem
//! primeiro. Por isso, o tamanho do arquivo precisa ser múltiplo de dois.
//!
//! A sequência abaixo representa as palavras `10`, `7` e `14`:
//!
//! ```text
//! 0A 00 07 00 0E 00
//! ```
//!
//! Se o arquivo terminar com um byte solto, a conversão falha com
//! [`SimulationError::MisalignedObjectFile`], porque não há uma palavra de 16
//! bits completa para carregar.
//!
//! ## Modelo de execução
//!
//! A máquina simulada mantém:
//! - memória de 65.536 palavras;
//! - acumulador assinado de 16 bits;
//! - contador de programa (`pc`) como índice de memória.
//!
//! A cada passo, `Processor::run` busca a palavra em `memory[pc]`, tenta
//! decodificá-la como `Opcode` e executa a instrução. Operandos de instrução
//! são endereços: em `LOAD X`, a palavra após o opcode contém o endereço de
//! `X`, e o valor carregado vem de `memory[X]`.
//!
//! ## Aritmética
//!
//! O acumulador usa [`SignedWord`] para que instruções como `JMPN` observem o
//! sinal do resultado. As operações aritméticas são circulares (`wrapping_*`),
//! modelando uma máquina real de 16 bits: overflow não é erro de simulação, e
//! sim preservação dos bits resultantes no registrador.
//!
//! ## Entrada e saída
//!
//! `INPUT` lê um valor assinado de 16 bits da entrada padrão e grava esse valor
//! no endereço indicado pelo operando. `OUTPUT` lê a palavra do endereço
//! indicado, interpreta seus bits como [`SignedWord`] e escreve o valor na
//! saída padrão.
//!
//! ## Política de erros
//!
//! Erros de execução são reportados por [`SimulationError`]. O simulador evita
//! `panic` para comportamentos causados pelo programa simulado, como opcode
//! inválido, divisão por zero, arquivo desalinhado ou contador de programa fora
//! da memória.

mod processor;

use std::{fs, io, path::Path};

use processor::Processor;

use crate::{
    assembler::{SignedWord, Word},
    assembly::one_pass::ObjectProgram,
    errors::SimulationError,
    language::instructions::Opcode,
};

/// Simula um arquivo `.obj`.
///
/// A função lê o arquivo, converte sua sequência binária de palavras em um
/// programa objeto, cria um processador limpo, carrega o programa na memória e
/// executa até `STOP` ou erro de simulação.
///
/// # Parâmetros
/// - `path`: caminho do arquivo objeto binário.
///
/// # Retorno
/// - `Ok(())` quando o programa termina com `STOP`;
/// - `Err(SimulationError)` quando o arquivo não pode ser carregado ou a
///   execução encontra uma falha.
///
/// # Erros
/// Retorna erro se a leitura do arquivo falhar, se o arquivo tiver quantidade
/// ímpar de bytes, se o programa não couber na memória ou se a execução
/// produzir um diagnóstico de simulação.
pub fn simulate_obj_file(path: impl AsRef<Path>) -> Result<(), SimulationError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|err| SimulationError::ObjectFileRead {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;
    let program = object_program_from_obj_bytes(&bytes)?;

    simulate_object(program)
}

/// Simula um programa objeto já materializado.
///
/// # Parâmetros
/// - `program`: palavras de máquina já resolvidas pela montagem.
///
/// # Retorno
/// - `Ok(())` quando a execução encontra `STOP`;
/// - `Err(SimulationError)` quando o carregamento ou a execução falha.
pub(crate) fn simulate_object(program: ObjectProgram) -> Result<(), SimulationError> {
    let mut processor = Processor::new();
    processor.load(program)?;
    processor.run()
}

/// Converte o conteúdo binário de um `.obj` em [`ObjectProgram`].
///
/// O arquivo é interpretado como uma sequência de [`Word`] em little-endian.
/// Cada par de bytes forma uma palavra da arquitetura alvo.
///
/// # Parâmetros
/// - `bytes`: conteúdo bruto do arquivo `.obj`.
///
/// # Retorno
/// - [`ObjectProgram`] com as palavras decodificadas na ordem em que aparecem
///   no arquivo.
///
/// # Erros
/// Retorna [`SimulationError::MisalignedObjectFile`] quando `bytes.len()` não é
/// múltiplo do tamanho de uma [`Word`].
pub(crate) fn object_program_from_obj_bytes(
    bytes: &[u8],
) -> Result<ObjectProgram, SimulationError> {
    let chunks = bytes.chunks_exact(Word::BITS as usize / 8);
    let remainder = chunks.remainder();

    if !remainder.is_empty() {
        return Err(SimulationError::MisalignedObjectFile { bytes: bytes.len() });
    }

    let words = chunks
        .map(|chunk| Word::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    Ok(ObjectProgram { words })
}

impl Processor {
    /// Executa o programa carregado na memória.
    ///
    /// O método interpreta a memória a partir de `pc = 0`, decodifica opcodes
    /// com [`Opcode::try_from`] e atualiza o contador de programa conforme o
    /// tamanho ou o destino da instrução executada.
    ///
    /// A aritmética do acumulador usa operações circulares (`wrapping_*`) para
    /// modelar uma máquina real de 16 bits: quando uma operação ultrapassa o
    /// intervalo de [`SignedWord`], os bits resultantes permanecem no
    /// registrador e passam a ser reinterpretados com sinal.
    ///
    /// # Retorno
    /// - `Ok(())` quando a instrução `STOP` é executada;
    /// - `Err(SimulationError)` quando a execução encontra uma falha.
    ///
    /// # Erros
    /// Pode retornar erro para opcode inválido, divisão por zero, endereço de
    /// instrução fora da memória ou entrada inválida em `INPUT`.
    pub(in crate::simulator) fn run(&mut self) -> Result<(), SimulationError> {
        loop {
            let pc = self.pc;
            let opcode = Opcode::try_from(self.fetch_word(pc)?).map_err(|err| {
                SimulationError::InvalidOpcode {
                    opcode: err.opcode,
                    pc,
                }
            })?;

            match opcode {
                Opcode::Add => {
                    let addr = self.operand_address(1)?;
                    self.acc = self.acc.wrapping_add(self.read_signed(addr)?);
                    self.pc += 2;
                }
                Opcode::Sub => {
                    let addr = self.operand_address(1)?;
                    self.acc = self.acc.wrapping_sub(self.read_signed(addr)?);
                    self.pc += 2;
                }
                Opcode::Mul => {
                    let addr = self.operand_address(1)?;
                    self.acc = self.acc.wrapping_mul(self.read_signed(addr)?);
                    self.pc += 2;
                }
                Opcode::Div => {
                    let addr = self.operand_address(1)?;
                    let divisor = self.read_signed(addr)?;
                    if divisor == 0 {
                        return Err(SimulationError::DivisionByZero { pc });
                    }
                    self.acc = self.acc.wrapping_div(divisor);
                    self.pc += 2;
                }
                Opcode::Jmp => self.pc = self.operand_address(1)?,
                Opcode::Jmpn => {
                    let target = self.operand_address(1)?;
                    if self.acc < 0 {
                        self.pc = target;
                    } else {
                        self.pc += 2;
                    }
                }
                Opcode::Jmpp => {
                    let target = self.operand_address(1)?;
                    if self.acc > 0 {
                        self.pc = target;
                    } else {
                        self.pc += 2;
                    }
                }
                Opcode::Jmpz => {
                    let target = self.operand_address(1)?;
                    if self.acc == 0 {
                        self.pc = target;
                    } else {
                        self.pc += 2;
                    }
                }
                Opcode::Copy => {
                    let src = self.operand_address(1)?;
                    let dst = self.operand_address(2)?;
                    let value = self.fetch_word(src)?;
                    self.memory[dst] = value;
                    self.pc += 3;
                }
                Opcode::Load => {
                    let addr = self.operand_address(1)?;
                    self.acc = self.read_signed(addr)?;
                    self.pc += 2;
                }
                Opcode::Store => {
                    let addr = self.operand_address(1)?;
                    self.memory[addr] = self.acc as Word;
                    self.pc += 2;
                }
                Opcode::Input => {
                    let addr = self.operand_address(1)?;
                    let mut input = String::new();
                    io::stdin()
                        .read_line(&mut input)
                        .expect("Falha ao ler a linha");
                    let input = input
                        .trim()
                        .parse::<SignedWord>()
                        .map_err(|_| SimulationError::InvalidInputType)?;

                    self.memory[addr] = input as Word;
                    self.pc += 2;
                }
                Opcode::Output => {
                    let addr = self.operand_address(1)?;
                    println!("{}", self.read_signed(addr)?);
                    self.pc += 2;
                }
                Opcode::Stop => return Ok(()),
            }
        }
    }

    /// Lê uma palavra da memória.
    ///
    /// # Parâmetros
    /// - `address`: endereço absoluto de memória.
    ///
    /// # Retorno
    /// - palavra armazenada no endereço solicitado.
    ///
    /// # Erros
    /// Retorna [`SimulationError::ProgramCounterOutOfBounds`] quando o endereço
    /// não existe na memória simulada.
    fn fetch_word(&self, address: usize) -> Result<Word, SimulationError> {
        self.memory
            .get(address)
            .copied()
            .ok_or(SimulationError::ProgramCounterOutOfBounds { pc: address })
    }

    /// Lê o operando de uma instrução como endereço.
    ///
    /// # Parâmetros
    /// - `offset`: deslocamento em relação ao `pc` atual. Para instruções de um
    ///   operando, o valor usual é `1`.
    ///
    /// # Retorno
    /// - endereço armazenado na palavra de operando.
    ///
    /// # Erros
    /// Propaga erro se a palavra do operando estiver fora da memória.
    fn operand_address(&self, offset: usize) -> Result<usize, SimulationError> {
        self.fetch_word(self.pc + offset).map(usize::from)
    }

    /// Lê uma palavra da memória e a interpreta como valor assinado.
    ///
    /// # Parâmetros
    /// - `address`: endereço absoluto de memória.
    ///
    /// # Retorno
    /// - bits da palavra reinterpretados como [`SignedWord`].
    ///
    /// # Erros
    /// Propaga erro se `address` estiver fora da memória simulada.
    fn read_signed(&self, address: usize) -> Result<SignedWord, SimulationError> {
        self.fetch_word(address).map(|word| word as SignedWord)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
