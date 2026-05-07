//! Representação intermediária produzida pelo parser da linguagem montável.
//!
//! Este módulo define os nós da linguagem assembly final reconhecidos pelo
//! parser. Ele não modela o programa inteiro nem a estrutura de linhas por
//! seção; esses envelopes vivem em [`crate::parser::types`].
//!
//! Papel no pipeline:
//! 1. o pré-processador remove diretivas próprias (`MACRO`, `EQU`, `IF` e
//!    `SECTION`) e entrega linhas lógicas por seção;
//! 2. o parser valida rótulos, mnemônicos, operandos e diretivas de dados;
//! 3. a montagem consome esta IR para calcular endereços, resolver símbolos e
//!    emitir palavras de máquina.
//!
//! Contrato dos tipos:
//! - preservam símbolos internados (`Symbol`) em vez de strings;
//! - guardam `Span` nos pontos relevantes para diagnósticos posteriores;
//! - representam sintaxe já reconhecida, mas ainda sem resolução de endereço;
//! - mantêm literais numéricos em forma sintática, deixando conversão e range
//!   checking para a montagem.

use crate::{interner::Symbol, language::instructions::Mnemonic, lexer::Span};

/// Definição de rótulo em uma linha assembly.
///
/// Forma canônica:
///
/// ```text
/// LOOP: LOAD VALUE
/// VALUE: SPACE
/// ```
///
/// Relação com a linguagem:
/// - rótulos nomeiam endereços de instruções ou dados;
/// - o parser guarda apenas o símbolo e o span;
/// - a montagem decide o endereço concreto usando o contador de posição.
pub(crate) struct LabelDef {
    /// Símbolo internado do rótulo definido, sem o `:`.
    pub name: Symbol,
    /// Span do identificador do rótulo.
    pub span: Span,
}

/// Referência simbólica usada como operando de endereço.
///
/// Forma canônica:
///
/// ```text
/// LOAD VALUE
/// JMP LOOP
/// ```
///
/// Diferente de [`LabelDef`], este tipo não define um símbolo: ele referencia
/// um rótulo que deve ser resolvido pela montagem.
pub(crate) struct SymbolRef {
    /// Símbolo internado referenciado pelo operando.
    pub name: Symbol,
    /// Span do identificador referenciado.
    pub span: Span,
}

/// Sinal explícito de um literal numérico.
///
/// O lexer emite `+` e `-` como tokens independentes. O parser usa este enum
/// para preservar a forma assinada do literal sem converter o valor ainda.
pub(crate) enum Sign {
    /// Prefixo `+`.
    Plus,
    /// Prefixo `-`.
    Minus,
}

/// Literal numérico em forma sintática.
///
/// Formas canônicas:
///
/// ```text
/// CONST 10
/// CONST +10
/// CONST -10
/// ```
///
/// Relação com a linguagem:
/// - `sign` representa a presença explícita de `+` ou `-`;
/// - `value` guarda o token numérico internado sem sinal;
/// - a conversão para palavra da arquitetura ocorre na montagem.
pub(crate) struct NumberLiteral {
    /// Sinal explícito do literal, quando presente.
    pub sign: Option<Sign>,
    /// Símbolo internado do token numérico sem sinal.
    pub value: Symbol,
    /// Span do literal completo.
    pub span: Span,
}

/// Operando que denota um endereço na máquina hipotética.
///
/// A linguagem aceita endereços simbólicos diretos e, para instruções que
/// acessam memória, endereços com deslocamento a partir de um rótulo.
///
/// Formas canônicas:
///
/// ```text
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
        /// Span do operando inteiro, incluindo base e deslocamento.
        span: Span,
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
/// ```text
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
        /// Span da instrução inteira.
        span: Span,
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
        /// Span da instrução inteira.
        span: Span,
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
        /// Span da instrução inteira.
        span: Span,
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
/// ```text
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
        /// Span da diretiva inteira.
        span: Span,
    },
    /// Emite uma palavra constante na seção de dados.
    Const {
        /// Valor literal a ser emitido.
        value: NumberLiteral,
        /// Span da diretiva inteira.
        span: Span,
    },
}
