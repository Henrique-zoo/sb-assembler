//! Tipos de suporte do estágio de pré-processamento.
//!
//! Este módulo concentra estruturas de estado e dados compartilhados entre:
//! - detecção de diretivas (`detection`);
//! - parsing sintático (`parser`);
//! - execução semântica (`execute`);
//! - orquestração do fluxo por linha (`preprocessor::mod`).
//!
//! Objetivo:
//! - manter o modelo interno do pré-processador em um único ponto;
//! - facilitar evolução do pipeline sem espalhar tipos por submódulos.

use std::{collections::HashMap, iter::Peekable, vec::IntoIter};

use crate::{
    interner::{Interner, Symbol},
    language::{
        LanguageSymbols,
        numeric_literals::{self, NumberParseError, NumberSign},
    },
    lexer::{Span, Token, TokenKind},
    preprocessor::ir::Macro,
};

/// Estrutura principal de estado do pré-processador.
///
/// Mantém:
/// - tabelas semânticas (macros e aliases `EQU`);
/// - contexto de seção atual;
/// - referência ao vocabulário internado da linguagem;
/// - metadados de diagnóstico por `NodeId` (`node_spans`).
#[derive(Debug)]
pub(crate) struct Preprocessor<'language> {
    /// Tabela de macros definidas (`nome -> definição`).
    pub(super) macros: HashMap<Symbol, Macro>,
    /// Tabela de aliases `EQU` (`alias -> substituição`).
    ///
    /// A forma armazenada preserva sinal e símbolo numérico, permitindo tanto
    /// avaliação semântica (`IF`) quanto reemissão de tokens (`DATA`) sem
    /// reconstruir texto a partir de um inteiro já convertido.
    pub(super) equs: HashMap<Symbol, EquReplacement>,
    /// Seção de saída atualmente ativa.
    pub(super) current_section: Section,
    /// Vocabulário internado da linguagem.
    pub(super) language_symbols: &'language LanguageSymbols,
    /// Tabela lateral de spans da IR indexada por `NodeId`.
    pub(super) node_spans: Vec<Span>,
}

/// Programa emitido pelo pré-processador, já separado por seção montável.
///
/// Diretivas próprias do pré-processador (`MACRO`, `EQU`, `IF` e `SECTION`) não
/// são preservadas como linhas nessa estrutura. `SECTION TEXT` e `SECTION DATA`
/// apenas controlam em qual vetor as linhas seguintes serão acumuladas.
#[derive(Debug, Clone)]
pub(crate) struct PreprocessedProgram {
    /// Linhas da seção de texto/instruções, após expansão de macros e aplicação
    /// de `IF`.
    pub text: Vec<LogicalLine>,
    /// Linhas da seção de dados, preservadas na ordem em que aparecem dentro de
    /// `SECTION DATA`.
    pub data: Vec<LogicalLine>,
}

/// Seção lógica corrente do fonte durante o pré-processamento.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Section {
    /// Ainda não entrou em seção explícita. Linhas comuns nesse estado não
    /// entram no programa preprocessado.
    None,
    /// Seção de texto/instruções.
    Text,
    /// Seção de dados.
    Data,
}

/// Linha lógica produzida a partir do fluxo de tokens do lexer.
///
/// `content` contém apenas tokens da linha (sem `NewLine`) e `terminator`
/// guarda a quebra que encerra a linha.
#[derive(Debug, Clone)]
pub(crate) struct LogicalLine {
    /// Conteúdo da linha lógica, sem token terminador.
    pub content: Vec<Token>,
    /// Quebra de linha real ou sintética que encerra a linha.
    pub terminator: Token,
}

impl LogicalLine {
    /// Retorna o [`Span`] da linha toda.
    pub(in crate::preprocessor) fn span(&self) -> Span {
        match (self.content.first(), self.content.last()) {
            (Some(first), Some(last)) => {
                let end = last.span.pos.saturating_add(last.span.len);

                Span {
                    pos: first.span.pos,
                    line: first.span.line,
                    column: first.span.column,
                    len: end.saturating_sub(first.span.pos)
                }
            }
            _ => self.terminator.span,
        }
    }
}

/// Iterador que agrupa tokens do lexer em [`LogicalLine`]s.
///
/// Diferente de uma coleta antecipada em `Vec<LogicalLine>`, este tipo só monta
/// a próxima linha quando o orquestrador pede `next()`. Isso evita percorrer e
/// materializar todas as linhas antes do pré-processamento e mantém o mesmo
/// cursor compartilhado por diretivas multilinha como `MACRO` e `IF`.
pub(in crate::preprocessor) struct LogicalLines {
    tokens: IntoIter<Token>,
    current_line: Vec<Token>,
}

/// Iterador usado pelo orquestrador para percorrer linhas lógicas sob demanda.
pub(in crate::preprocessor) type LogicalLineIter = Peekable<LogicalLines>;

/// Conversão de um fluxo de tokens para um iterador de linhas lógicas.
///
/// O nome `into_logical_lines` deixa explícito que o vetor de tokens é consumido
/// e que o resultado não é uma coleção materializada, mas um iterador que produz
/// [`LogicalLine`]s sob demanda.
pub(in crate::preprocessor) trait IntoLogicalLines {
    /// Consome `self` e devolve um iterador de [`LogicalLine`]s.
    fn into_logical_lines(self) -> LogicalLines;
}

impl IntoLogicalLines for Vec<Token> {
    fn into_logical_lines(self) -> LogicalLines {
        LogicalLines {
            tokens: self.into_iter(),
            current_line: Vec::new(),
        }
    }
}

impl Iterator for LogicalLines {
    type Item = LogicalLine;

    fn next(&mut self) -> Option<Self::Item> {
        for token in self.tokens.by_ref() {
            if matches!(token.kind, TokenKind::NewLine) {
                return Some(LogicalLine {
                    content: std::mem::take(&mut self.current_line),
                    terminator: token,
                });
            }

            self.current_line.push(token);
        }

        None
    }
}

/// Forma reemitível de um alias `EQU`.
///
/// `EQU` pode ser consumido de duas maneiras:
/// - como valor semântico em `IF`;
/// - como sequência de tokens quando aparece como operando de `CONST`/`SPACE`.
///
/// Por isso a tabela do pré-processador guarda a forma sintática normalizada,
/// não um inteiro já convertido. Cada consumidor converte ou materializa tokens
/// no momento em que conhece o contexto.
#[derive(Debug, Clone, Copy)]
pub(in crate::preprocessor) enum EquReplacement {
    /// Literal sem sinal explícito.
    Unsigned {
        /// Símbolo internado do token numérico.
        number: Symbol,
    },
    /// Literal com sinal explícito.
    Signed {
        /// Sinal preservado do literal.
        sign: NumberSign,
        /// Símbolo internado do token numérico sem o sinal.
        number: Symbol,
    },
}

impl EquReplacement {
    /// Cria uma substituição a partir de um literal numérico da linguagem.
    pub(in crate::preprocessor) fn from_numeric_literal(
        literal: crate::language::numeric_literals::NumericLiteral,
    ) -> Self {
        match literal.sign {
            Some(sign) => Self::Signed {
                sign,
                number: literal.symbol(),
            },
            None => Self::Unsigned {
                number: literal.symbol(),
            },
        }
    }

    /// Retorna o símbolo do token numérico associado ao alias.
    pub(in crate::preprocessor) fn number_symbol(self) -> Symbol {
        match self {
            Self::Unsigned { number } | Self::Signed { number, .. } => number,
        }
    }

    /// Avalia a substituição como inteiro para uso em `IF`.
    pub(in crate::preprocessor) fn evaluate(
        self,
        interner: &Interner,
    ) -> Result<i32, NumberParseError> {
        match self {
            Self::Unsigned { number } => {
                numeric_literals::parse_unsigned_symbol(number, interner).map(i32::from)
            }
            Self::Signed { sign, number } => {
                numeric_literals::parse_signed_symbol(sign, number, interner).map(i32::from)
            }
        }
    }

    /// Materializa a substituição como tokens usando o span do ponto de uso.
    pub(in crate::preprocessor) fn to_tokens_with_span(self, span: Span) -> Vec<Token> {
        match self {
            Self::Unsigned { number } => vec![Token::new(TokenKind::Number(number), span)],
            Self::Signed { sign, number } => {
                let sign_kind = match sign {
                    NumberSign::Plus => TokenKind::Plus,
                    NumberSign::Minus => TokenKind::Minus,
                };

                vec![
                    Token::new(sign_kind, span),
                    Token::new(TokenKind::Number(number), span),
                ]
            }
        }
    }
}
