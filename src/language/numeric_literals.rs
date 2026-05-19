//! Conversão de literais numéricos da linguagem assembly.
//!
//! Este módulo concentra a regra comum de interpretação dos lexemas numéricos
//! reconhecidos pelo lexer. Ele não pertence ao pré-processador nem à montagem:
//! decimal, hexadecimal (`0X...`) e binário (`0B...`) são formatos da
//! linguagem, consumidos por mais de um estágio.

use crate::{
    assembler::{SignedWord, Word},
    interner::{Interner, Symbol},
};

/// Erro técnico de parsing de literal numérico internado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberParseError {
    /// Lexema não representa número válido nos formatos aceitos.
    InvalidNumber,
    /// Valor numérico ultrapassa o limite do tipo alvo do parsing.
    Overflow,
}

/// Sinal explícito associado a um literal numérico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberSign {
    /// Prefixo `+`.
    Plus,
    /// Prefixo `-`.
    Minus,
}

/// Literal numérico em forma sintática.
///
/// Este tipo representa o número como a linguagem o reconhece, antes de
/// qualquer conversão para [`Word`] ou [`SignedWord`]. Ele não carrega `Span`
/// nem `NodeId`: cada estágio deve embrulhar o literal com seus próprios
/// metadados de diagnóstico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NumericLiteral {
    /// Sinal explícito do literal, quando presente.
    pub sign: Option<NumberSign>,
    /// Símbolo internado do token numérico sem o sinal.
    pub symbol: Symbol,
}

impl NumericLiteral {
    /// Constrói literal sem sinal explícito.
    pub(crate) fn unsigned(symbol: Symbol) -> Self {
        Self { sign: None, symbol }
    }

    /// Constrói literal com sinal explícito.
    pub(crate) fn signed(sign: NumberSign, symbol: Symbol) -> Self {
        Self {
            sign: Some(sign),
            symbol,
        }
    }

    /// Retorna o símbolo internado associado ao token numérico.
    pub(crate) fn symbol(self) -> Symbol {
        self.symbol
    }
}

impl NumberSign {
    /// Retorna o lexema textual do sinal.
    fn as_str(self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
        }
    }
}

/// Converte um símbolo internado de literal não-assinado para [`Word`].
///
/// Formatos aceitos:
/// - decimal (`42`);
/// - hexadecimal com prefixo `0X`;
/// - binário com prefixo `0B`.
pub(crate) fn parse_unsigned_symbol(
    sym: Symbol,
    interner: &Interner,
) -> Result<Word, NumberParseError> {
    let lexeme = interner
        .get_str(sym)
        .ok_or(NumberParseError::InvalidNumber)?;

    let parsed = if let Some(hex) = lexeme.strip_prefix("0X") {
        u32::from_str_radix(hex, 16).map_err(|_| NumberParseError::InvalidNumber)?
    } else if let Some(bin) = lexeme.strip_prefix("0B") {
        u32::from_str_radix(bin, 2).map_err(|_| NumberParseError::InvalidNumber)?
    } else {
        lexeme
            .parse::<u32>()
            .map_err(|_| NumberParseError::InvalidNumber)?
    };

    Word::try_from(parsed).map_err(|_| NumberParseError::Overflow)
}

/// Converte um símbolo internado de literal assinado para [`SignedWord`].
///
/// O sinal é informado separadamente porque o lexer preserva `+`/`-` como
/// tokens próprios, e o parser monta o literal completo de acordo com o
/// contexto sintático.
pub(crate) fn parse_signed_symbol(
    sign: NumberSign,
    sym: Symbol,
    interner: &Interner,
) -> Result<SignedWord, NumberParseError> {
    let num_str = interner
        .get_str(sym)
        .ok_or(NumberParseError::InvalidNumber)?;

    let sign_str = sign.as_str();

    let parsed = if let Some(hex) = num_str.strip_prefix("0X") {
        i32::from_str_radix(&(sign_str.to_string() + hex), 16)
            .map_err(|_| NumberParseError::InvalidNumber)?
    } else if let Some(bin) = num_str.strip_prefix("0B") {
        i32::from_str_radix(&(sign_str.to_string() + bin), 2)
            .map_err(|_| NumberParseError::InvalidNumber)?
    } else {
        (sign_str.to_string() + num_str)
            .parse::<i32>()
            .map_err(|_| NumberParseError::InvalidNumber)?
    };

    SignedWord::try_from(parsed).map_err(|_| NumberParseError::Overflow)
}
