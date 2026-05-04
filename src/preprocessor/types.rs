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

use std::collections::HashMap;

use crate::{
    interner::Symbol,
    lexer::{Token, TokenKind},
    preprocessor::ir::Macro,
};

/// Símbolos internados das keywords reconhecidas pelo pré-processador.
///
/// Estes campos são inicializados em `Preprocessor::new` para evitar
/// internamento repetido ao longo do parsing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Keywords {
    /// Keyword `SECTION`.
    pub section_kw: Symbol,
    /// Keyword `TEXT`.
    pub text_kw: Symbol,
    /// Keyword `DATA`.
    pub data_kw: Symbol,
    /// Keyword `MACRO`.
    pub macro_kw: Symbol,
    /// Keyword `ENDMACRO`.
    pub endmacro_kw: Symbol,
    /// Keyword `EQU`.
    pub equ_kw: Symbol,
    /// Keyword `IF`.
    pub if_kw: Symbol,
}

/// Símbolos internados de tokens fixos de pontuação/operadores.
///
/// Esses símbolos são usados principalmente para diagnósticos sintáticos
/// (`MissingToken`/`UnexpectedToken`) sem exigir internamento ad-hoc durante
/// o parsing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FixedSymbols {
    /// Símbolo de `,`.
    pub comma: Symbol,
    /// Símbolo de `:`.
    pub colon: Symbol,
    /// Símbolo de `&`.
    pub ampersand: Symbol,
    /// Símbolo de `+`.
    pub plus: Symbol,
    /// Símbolo de `-`.
    pub minus: Symbol,
}

impl TokenKind {
    /// Converte `TokenKind` em símbolo usando o contexto de [`FixedSymbols`].
    ///
    /// Retorna:
    /// - `Some(sym)` para `Ident`/`Number` e tokens fixos mapeados;
    /// - `None` para variantes sem símbolo fixo (`NewLine`, `Eof`).
    pub(crate) fn to_sym(&self, fixed_symbols: &FixedSymbols) -> Option<Symbol> {
        match self {
            TokenKind::Ident(sym) | TokenKind::Number(sym) => Some(*sym),
            TokenKind::Ampersand => Some(fixed_symbols.ampersand),
            TokenKind::Comma => Some(fixed_symbols.comma),
            TokenKind::Colon => Some(fixed_symbols.colon),
            TokenKind::Plus => Some(fixed_symbols.plus),
            TokenKind::Minus => Some(fixed_symbols.minus),
            TokenKind::NewLine | TokenKind::Eof => None,
        }
    }
}

impl Token {
    /// Atalho para converter o token atual em símbolo.
    ///
    /// Delega para [`TokenKind::to_sym`] usando o `kind` do token.
    pub(crate) fn to_sym(&self, fixed_symbols: &FixedSymbols) -> Option<Symbol> {
        self.kind.to_sym(fixed_symbols)
    }
}

/// Estrutura principal de estado do pré-processador.
///
/// Mantém:
/// - tabelas semânticas (macros e aliases `EQU`);
/// - contexto de seção atual;
/// - símbolos internados de keywords e tokens fixos.
#[derive(Debug)]
pub(crate) struct Preprocessor {
    /// Tabela de macros definidas (`nome -> definição`).
    pub(super) macros: HashMap<Symbol, Macro>,
    /// Tabela de aliases `EQU` (`alias -> valor`).
    pub(super) equs: HashMap<Symbol, Symbol>,
    /// Seção de saída atualmente ativa.
    pub(super) current_section: Section,
    /// Símbolos internados das keywords da linguagem.
    pub(super) keywords: Keywords,
    /// Símbolos internados de tokens fixos usados em diagnósticos.
    pub(super) fixed_symbols: FixedSymbols,
}

/// Seção lógica corrente do fonte durante o pré-processamento.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Section {
    /// Ainda não entrou em seção explícita.
    None,
    /// Seção de texto/instruções.
    Text,
    /// Seção de dados.
    Data,
}

/// Linha lógica produzida a partir do fluxo de tokens do lexer.
///
/// `content` contém apenas tokens da linha (sem `NewLine`/`Eof`) e
/// `terminator` guarda o separador original da linha no fonte.
pub(super) struct LogicalLine {
    /// Conteúdo da linha lógica, sem token terminador.
    pub content: Vec<Token>,
    /// Separador original da linha (`NewLine` ou `Eof`).
    pub terminator: Token,
}
