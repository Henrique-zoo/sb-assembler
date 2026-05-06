//! Utilitários de parsing numérico do pré-processador.
//!
//! Este módulo concentra a conversão de literais internados em valores da
//! arquitetura alvo e os erros técnicos associados a essa conversão.

use crate::{
    assembler::{SignedWord, Word},
    interner::{Interner, Symbol},
    preprocessor::ir::Number,
};

/// Erro técnico de parsing de literal numérico internado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::preprocessor) enum NumberParseError {
    /// Lexema não representa número válido nos formatos aceitos.
    InvalidNumber,
    /// Valor numérico ultrapassa o limite do tipo alvo do parsing.
    Overflow,
}

/// Converte um [`Number::Unsigned`] para [`Word`].
///
/// Pré-condição:
/// - `number` deve ser `Number::Unsigned`; `Number::Signed` é tratado como
///   estado impossível e dispara `unreachable!()`.
///
/// Formatos aceitos para o lexema base:
/// - decimal (`42`);
/// - hexadecimal com prefixo `0X`;
/// - binário com prefixo `0B`.
///
/// Observação:
/// - atualmente os prefixos reconhecidos são apenas em maiúsculo (`0X`/`0B`).
///
/// Retorna:
/// - `Ok(value)` quando o número não-assinado é válido para [`Word`];
/// - `Err(NumberParseError::InvalidNumber)` para lexema inválido/ausente;
/// - `Err(NumberParseError::Overflow)` para estouro do intervalo de `u16`.
pub(in crate::preprocessor) fn parse_unsigned_number(
    number: &Number,
    interner: &Interner,
) -> Result<Word, NumberParseError> {
    match number {
        Number::Unsigned { sym } => parse_unsigned_number_symbol(*sym, interner),
        Number::Signed { .. } => unreachable!(),
    }
}

/// Converte símbolo internado de literal numérico para [`Word`].
///
/// Formatos aceitos:
/// - decimal (`42`);
/// - hexadecimal com prefixo `0X`;
/// - binário com prefixo `0B`.
///
/// Observação:
/// - atualmente os prefixos reconhecidos são apenas em maiúsculo (`0X`/`0B`).
///
/// Retorna:
/// - `Ok(value)` quando o lexema associado a `sym` é um número válido na
///   faixa de [`Word`];
/// - `Err(NumberParseError::InvalidNumber)` quando o símbolo não existe no
///   interner ou o texto não é parseável;
/// - `Err(NumberParseError::Overflow)` quando o número estoura `u16`.
fn parse_unsigned_number_symbol(
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

/// Converte um [`Number::Signed`] para [`SignedWord`].
///
/// Pré-condição:
/// - `number` deve ser `Number::Signed`; `Number::Unsigned` é tratado como
///   estado impossível e dispara `unreachable!()`.
///
/// Formatos aceitos para o lexema base (sem sinal):
/// - decimal (`10`);
/// - hexadecimal com prefixo `0X`;
/// - binário com prefixo `0B`.
///
/// Observação:
/// - para literais assinados, o sinal é aplicado apenas após detectar a base
///   do número em `num_str`.
/// - isso permite parsear corretamente literais como `+0X10` e `-0B11`.
///
/// Retorna:
/// - `Ok(value)` quando o número assinado é válido para [`SignedWord`];
/// - `Err(NumberParseError::InvalidNumber)` para lexema inválido/ausente;
/// - `Err(NumberParseError::Overflow)` para estouro do intervalo de `i16`.
pub(in crate::preprocessor) fn parse_signed_number(
    number: &Number,
    interner: &Interner,
) -> Result<SignedWord, NumberParseError> {
    let (sig_str, num_str) = match number {
        Number::Signed { sign, sym } => {
            let num_str = interner
                .get_str(*sym)
                .ok_or_else(|| NumberParseError::InvalidNumber)?;
            (<&str>::from(sign).to_string(), num_str)
        }
        Number::Unsigned { .. } => unreachable!(),
    };

    let parsed = if let Some(hex) = num_str.strip_prefix("0X") {
        i32::from_str_radix(&(sig_str + hex), 16).map_err(|_| NumberParseError::InvalidNumber)?
    } else if let Some(bin) = num_str.strip_prefix("0B") {
        i32::from_str_radix(&(sig_str + bin), 2).map_err(|_| NumberParseError::InvalidNumber)?
    } else {
        (sig_str + num_str)
            .parse::<i32>()
            .map_err(|_| NumberParseError::InvalidNumber)?
    };

    SignedWord::try_from(parsed).map_err(|_| NumberParseError::Overflow)
}
