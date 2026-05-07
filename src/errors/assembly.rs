//! Erros produzidos durante a montagem.
//!
//! Este módulo modela diagnósticos do estágio que consome a IR do parser e
//! emite palavras de máquina. Nesse ponto, a sintaxe já foi validada; os erros
//! aqui descrevem falhas semânticas de montagem, como resolução de símbolos,
//! conversão de literais e estouro de endereços.

use crate::{interner::Symbol, lexer::Span};

/// Contexto em que um literal numérico está sendo interpretado.
///
/// O mesmo [`crate::parser::ir::NumberLiteral`] pode aparecer em posições com
/// regras diferentes. Este enum permite que o diagnóstico preserve onde a
/// conversão falhou sem criar uma variante de erro para cada diretiva ou
/// operando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberContext {
    /// Valor emitido por uma diretiva `CONST`.
    ConstValue,
    /// Quantidade de palavras reservadas por uma diretiva `SPACE`.
    SpaceAmount,
    /// Deslocamento aplicado a um operando de endereço.
    AddressOffset,
}

/// Categorias de erro do estágio de montagem.
#[derive(Debug, Clone)]
pub(crate) enum AssemblyErrorKind {
    /// Uma linha da seção `DATA` não tem rótulo
    NoRotuleInDataLine,
    /// Um rótulo foi definido mais de uma vez.
    SymbolAlreadyDefined {
        /// Símbolo internado do rótulo redefinido.
        symbol: Symbol,
        /// Span da primeira definição conhecida.
        first_defined_at: Span,
    },
    /// Um símbolo referenciado não foi definido em nenhuma linha montável.
    UndefinedSymbol {
        /// Símbolo internado da referência não resolvida.
        symbol: Symbol,
    },
    /// Literal numérico não pôde ser convertido no contexto informado.
    InvalidNumber {
        /// Símbolo internado do token numérico.
        value: Symbol,
        /// Posição semântica em que o número estava sendo interpretado.
        context: NumberContext,
    },
    /// Literal numérico válido sintaticamente, mas fora do intervalo aceito.
    NumberOverflow {
        /// Símbolo internado do token numérico.
        value: Symbol,
        /// Posição semântica em que o número estava sendo interpretado.
        context: NumberContext,
    },
    /// Quantidade de `SPACE` semanticamente inválida.
    ///
    /// Usado para valores como zero ou números negativos, quando a diretiva
    /// precisa reservar uma quantidade positiva de palavras.
    InvalidSpaceAmount {
        /// Símbolo internado do token numérico da quantidade.
        value: Symbol,
    },
    /// Endereço calculado não cabe em uma palavra da arquitetura.
    AddressOverflow {
        /// Endereço calculado antes da conversão para
        /// [`crate::assembler::Word`].
        address: usize,
    },
}

/// Diagnóstico final emitido pela montagem.
///
/// Combina a categoria da falha com o trecho do fonte associado ao problema. O
/// `span` normalmente aponta para o rótulo, operando ou diretiva responsável
/// pelo erro.
#[derive(Debug, Clone)]
pub(crate) struct AssemblyError {
    /// Categoria específica da falha de montagem.
    pub kind: AssemblyErrorKind,
    /// Região do fonte associada ao problema.
    pub span: Span,
}
