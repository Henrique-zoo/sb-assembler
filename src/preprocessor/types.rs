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
    assembler::{SignedWord, Word},
    interner::Symbol,
    language::KeywordTable,
    lexer::{Span, Token, TokenKind},
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

/// Estrutura principal de estado do pré-processador.
///
/// Mantém:
/// - tabelas semânticas (macros e aliases `EQU`);
/// - contexto de seção atual;
/// - símbolos internados de keywords;
/// - metadados de diagnóstico por `NodeId` (`node_spans`).
#[derive(Debug)]
pub(crate) struct Preprocessor {
    /// Tabela de macros definidas (`nome -> definição`).
    pub(super) macros: HashMap<Symbol, Macro>,
    /// Tabela de aliases `EQU` (`alias -> valor`).
    ///
    /// O valor já é armazenado em forma semântica tipada ([`EquValue`]),
    /// preservando distinção entre números assinados e não-assinados.
    ///
    /// Isso evita perda de informação de sinal e reduz necessidade de reparsing
    /// textual em etapas posteriores.
    pub(super) equs: HashMap<Symbol, EquValue>,
    /// Seção de saída atualmente ativa.
    pub(super) current_section: Section,
    /// Símbolos internados das keywords da linguagem.
    pub(super) keywords: Keywords,
    /// Tabela de palavras reservadas da linguagem.
    pub(super) keyword_table: KeywordTable,
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

/// Valor semântico normalizado para aliases `EQU`.
///
/// Este enum modela o valor já resolvido de uma declaração `EQU`, mantendo
/// explicitamente a natureza assinada ou não-assinada do resultado.
///
/// Objetivo:
/// - preservar semântica de `+`/`-` sem depender do lexema original;
/// - evitar ambiguidades ao consumir aliases `EQU` em diretivas como `IF`.
#[derive(Debug, Clone, Copy)]
pub enum EquValue {
    /// Valor não-assinado de 16 bits.
    Unsigned(Word),
    /// Valor assinado de 16 bits.
    Signed(SignedWord),
}
