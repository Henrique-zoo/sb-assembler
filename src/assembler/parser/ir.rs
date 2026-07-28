//! Representação intermediária produzida pelo parser da linguagem montável.
//!
//! Este módulo define os nós da linguagem assembly final reconhecidos pelo
//! parser. Ele não modela o programa inteiro nem a estrutura de linhas por
//! seção; esses envelopes vivem em [`crate::assembler::parser::types`].
//!
//! Papel no pipeline:
//! 1. o pré-processador remove diretivas próprias (`MACRO`, `EQU`, `IF` e
//!    `SECTION`) e entrega linhas lógicas por seção;
//! 2. o parser valida rótulos, mnemônicos, operandos e diretivas de dados;
//! 3. a montagem consome esta IR para calcular endereços, resolver símbolos e
//!    emitir palavras de máquina.
//!
//! Contrato dos tipos:
//! - preservam símbolos internados ([`Symbol`]) em vez de strings;
//! - usam [`NodeId`] para referenciar spans guardados no envelope do parser;
//! - representam sintaxe já reconhecida, mas ainda sem resolução de endereço;
//! - mantêm literais numéricos em forma sintática, deixando conversão e range
//!   checking para a montagem.

use crate::{
    assembler::interner::Symbol,
    assembler::language::{instructions::Mnemonic, numeric_literals::NumericLiteral},
};

/// Identificador estável de um nó da IR do parser.
///
/// O [`NodeId`] desacopla os metadados de diagnóstico da representação principal
/// da IR. O mapeamento entre identificador e [`crate::assembler::lexer::Span`] fica no
/// envelope retornado pelo parser, em [`crate::assembler::parser::types::NodeSpans`].
///
/// Contrato:
/// - deve ser tratado como identificador opaco;
/// - só é significativo junto da tabela lateral que foi produzida pelo mesmo
///   parser;
/// - aponta para o span do nó inteiro, não necessariamente para um token único.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(
    /// Índice na tabela lateral de spans do parser.
    pub u32,
);

/// Definição de rótulo em uma linha assembly.
///
/// Forma canônica:
///
/// ```ignore
/// LOOP: LOAD VALUE
/// VALUE: SPACE
/// ```
///
/// Relação com a linguagem:
/// - rótulos nomeiam endereços de instruções ou dados;
/// - o parser guarda apenas o símbolo e o identificador do span;
/// - a montagem decide o endereço concreto usando o contador de posição.
pub(crate) struct LabelDef {
    /// Símbolo internado do rótulo definido, sem o `:`.
    pub name: Symbol,
    /// `NodeId` do identificador do rótulo.
    pub node_id: NodeId,
}

/// Referência simbólica usada como operando de endereço.
///
/// Forma canônica:
///
/// ```ignore
/// LOAD VALUE
/// JMP LOOP
/// ```
///
/// Diferente de [`LabelDef`], este tipo não define um símbolo: ele referencia
/// um rótulo que deve ser resolvido pela montagem.
pub(crate) struct SymbolRef {
    /// Símbolo internado referenciado pelo operando.
    pub name: Symbol,
    /// [`NodeId`] do identificador referenciado.
    pub node_id: NodeId,
}

/// Literal numérico parseado com metadado de diagnóstico.
///
/// Formas canônicas:
///
/// ```ignore
/// CONST 10
/// CONST +10
/// CONST -10
/// ```
///
/// Relação com a linguagem:
/// - `literal` preserva sinal opcional e símbolo internado em uma forma comum
///   da linguagem;
/// - `node_id` aponta para o literal completo no fonte por meio da tabela
///   lateral do parser;
/// - a conversão para palavra da arquitetura ocorre na montagem.
pub(crate) struct NumberLiteral {
    /// Literal numérico sintático compartilhado entre parser e preprocessador.
    pub literal: NumericLiteral,
    /// [`NodeId`] do literal completo.
    pub node_id: NodeId,
}

/// Operando que denota um endereço na máquina hipotética.
///
/// A linguagem aceita endereços simbólicos diretos e, para instruções que
/// acessam memória, endereços com deslocamento a partir de um rótulo.
///
/// Formas canônicas:
///
/// ```ignore
/// LOAD VALUE
/// LOAD TABLE + 2
/// STORE TABLE - 1
/// ```
///
/// A montagem resolve o símbolo base e aplica o deslocamento, quando houver.
pub(crate) enum AddressOperand {
    /// Endereço simbólico direto.
    Direct(SymbolRef),
    /// Endereço formado por rótulo base e deslocamento numérico.
    Offset {
        /// Símbolo base do endereço.
        base: SymbolRef,
        /// Deslocamento aplicado ao endereço base.
        offset: NumberLiteral,
        /// [`NodeId`] do operando inteiro, incluindo base e deslocamento.
        node_id: NodeId,
    },
}

/// Instrução parseada da seção `TEXT`.
///
/// A variante codifica a aridade sintática reconhecida pelo parser. O
/// mnemônico em si vem de [`Mnemonic`], definido na camada de linguagem, e já
/// não é mais apenas um identificador textual qualquer.
///
/// Formas canônicas:
///
/// ```ignore
/// STOP
/// LOAD VALUE
/// COPY SRC, DST
/// ```
pub(crate) enum Instruction {
    /// Instrução sem operando.
    ///
    /// Na ISA atual, esta forma corresponde a `STOP`.
    NoOperand {
        /// Mnemônico reconhecido da instrução.
        mnemonic: Mnemonic,
        /// [`NodeId`] da instrução inteira.
        node_id: NodeId,
    },
    /// Instrução com um operando de endereço.
    ///
    /// Cobre instruções como `ADD`, `SUB`, `JMP`, `LOAD`, `STORE`, `INPUT` e
    /// `OUTPUT`. Quando houver deslocamento, ele fica dentro de
    /// [`AddressOperand`], não como segundo operando da instrução.
    OneOperand {
        /// Mnemônico reconhecido da instrução.
        mnemonic: Mnemonic,
        /// Operando de endereço consumido pela instrução.
        operand: AddressOperand,
        /// [`NodeId`] da instrução inteira.
        node_id: NodeId,
    },
    /// Instrução com dois operandos de endereço.
    ///
    /// Na ISA atual, esta forma corresponde a `COPY SRC, DST`, em que o
    /// primeiro operando é a origem e o segundo é o destino.
    TwoOperands {
        /// Mnemônico reconhecido da instrução.
        mnemonic: Mnemonic,
        /// Primeiro operando de endereço.
        first: AddressOperand,
        /// Segundo operando de endereço.
        second: AddressOperand,
        /// [`NodeId`] da instrução inteira.
        node_id: NodeId,
    },
}

/// Diretiva montável da seção `DATA`.
///
/// Estas diretivas pertencem à linguagem assembly final, não ao
/// pré-processador. O parser reconhece sua sintaxe, e a montagem aplica o
/// efeito sobre memória e contador de posição.
///
/// Formas canônicas:
///
/// ```ignore
/// VALUE SPACE
/// TABLE SPACE 10
/// CONST_VALUE CONST 5
/// ```
pub(crate) enum DataDirective {
    /// Reserva uma ou mais palavras de memória.
    Space {
        /// Quantidade de palavras a reservar.
        ///
        /// Quando ausente, a diretiva reserva uma palavra.
        amount: Option<NumberLiteral>,
        /// [`NodeId`] da diretiva inteira.
        node_id: NodeId,
    },
    /// Emite uma palavra constante na seção de dados.
    Const {
        /// Valor literal a ser emitido.
        value: NumberLiteral,
        /// [`NodeId`] da diretiva inteira.
        node_id: NodeId,
    },
}
