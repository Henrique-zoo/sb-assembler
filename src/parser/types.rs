//! Tipos estruturais do parser.
//!
//! Este módulo concentra os envelopes usados pela API do parser: o programa
//! parseado, suas linhas por seção e o formato comum de linha com rótulo
//! opcional.
//!
//! Diferente de [`crate::parser::ir`], que modela construções da linguagem
//! (`Instruction`, `DataDirective`, operandos e literais), estes tipos descrevem
//! a organização do resultado do parser.

use crate::{
    lexer::Span,
    parser::ir::{DataDirective, Instruction, LabelDef},
};

/// Programa assembly já parseado e separado por seção.
///
/// Esta estrutura preserva a divisão produzida pelo pré-processador: linhas de
/// `SECTION TEXT` entram em `text`, e linhas de `SECTION DATA` entram em
/// `data`.
///
/// Contrato no pipeline:
/// - `text` contém apenas instruções da máquina hipotética;
/// - `data` contém apenas diretivas de dados montáveis (`SPACE` e `CONST`);
/// - a montagem percorre esses vetores para emitir código e dados.
pub(crate) struct ParsedProgram {
    /// Linhas parseadas da seção de texto.
    pub text: Vec<TextLine>,
    /// Linhas parseadas da seção de dados.
    pub data: Vec<DataLine>,
}

/// Linha parseada da seção `TEXT`.
///
/// Equivale a uma [`ParsedLine`] cujo corpo é uma [`Instruction`].
pub(crate) type TextLine = ParsedLine<Instruction>;

/// Linha parseada da seção `DATA`.
///
/// Equivale a uma [`ParsedLine`] cujo corpo é uma [`DataDirective`].
pub(crate) type DataLine = ParsedLine<DataDirective>;

/// Linha assembly parseada com rótulo opcional.
///
/// Forma geral da linguagem:
///
/// ```text
/// <LABEL>: <CORPO>
/// <CORPO>
/// ```
///
/// O tipo é genérico porque a seção define o tipo do corpo: em `TEXT`, o corpo
/// é instrução; em `DATA`, é diretiva de dados.
pub(crate) struct ParsedLine<T> {
    /// Rótulo definido pela linha, quando presente.
    ///
    /// O montador associa este símbolo ao endereço corrente antes de emitir o
    /// corpo da linha.
    pub label: Option<LabelDef>,
    /// Construção principal da linha, já validada para a seção correspondente.
    pub body: T,
    /// Span da linha inteira usada para diagnosticar erros que pertencem à
    /// declaração completa.
    pub span: Span,
}
