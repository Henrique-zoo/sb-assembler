//! Tipos estruturais do parser.
//!
//! Este módulo concentra os envelopes usados pela API do parser: o programa
//! parseado, suas linhas por seção e o formato comum de linha com rótulo
//! opcional.
//!
//! Diferente de [`crate::assembler::parser::ir`], que modela construções da linguagem
//! (`Instruction`, `DataDirective`, operandos e literais), estes tipos descrevem
//! a organização do resultado do parser.

use crate::{
    assembler::lexer::Span,
    assembler::parser::ir::{DataDirective, Instruction, LabelDef, NodeId},
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
/// - `node_spans` preserva os spans referenciados pela IR por meio de
///   [`NodeId`];
/// - a montagem percorre esses vetores para emitir código e dados.
pub(crate) struct ParsedProgram {
    /// Linhas parseadas da seção de texto.
    pub text: Vec<TextLine>,
    /// Linhas parseadas da seção de dados.
    pub data: Vec<DataLine>,
    /// Tabela lateral de spans dos nós da IR.
    pub node_spans: NodeSpans,
}

/// Tabela lateral de spans dos nós produzidos pelo parser.
///
/// A IR do parser carrega apenas [`NodeId`]. Esta estrutura é o ponto que
/// associa cada identificador ao [`Span`] original do fonte, permitindo que a
/// montagem emita diagnósticos precisos sem inserir spans diretamente nos nós.
#[derive(Debug, Default)]
pub(crate) struct NodeSpans {
    spans: Vec<Span>,
}

impl NodeSpans {
    /// Cria uma tabela lateral vazia.
    pub(crate) fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Registra o span de um nó e retorna o identificador correspondente.
    ///
    /// # Parâmetros
    /// - `span`: intervalo de fonte correspondente ao nó recém-construído.
    ///
    /// # Retorno
    /// - [`NodeId`] estável para esta tabela.
    pub(crate) fn alloc(&mut self, span: Span) -> NodeId {
        let id = NodeId(self.spans.len() as u32);
        self.spans.push(span);
        id
    }

    /// Resolve o [`Span`] associado a um [`NodeId`].
    ///
    /// Em um fluxo válido, todo identificador recebido aqui foi produzido por
    /// [`Self::alloc`] na mesma tabela. O fallback com [`Span::default`] é
    /// defensivo para manter o diagnóstico total mesmo diante de IR inválida.
    pub(crate) fn span_of(&self, node_id: NodeId) -> Span {
        self.spans
            .get(node_id.0 as usize)
            .copied()
            .unwrap_or_default()
    }
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
    /// `NodeId` da linha inteira, usado para diagnosticar erros que pertencem à
    /// declaração completa.
    pub node_id: NodeId,
}
