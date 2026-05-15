use super::*;

use crate::{
    interner::Interner,
    language::{
        instructions::InstructionSet,
        numeric_literals::{NumberSign, NumericLiteral},
    },
    parser::{
        ir::{NodeId, NumberLiteral},
        types::{ParsedLine, ParsedProgram},
    },
};

/// Cria um span sintético para testes unitários.
///
/// # Parâmetros
/// - `pos`: posição usada como offset e base para coluna.
///
/// # Retorno
/// - [`Span`] de uma palavra, suficiente para validar propagação de
///   diagnósticos nos testes do montador.
fn span(pos: usize) -> Span {
    Span {
        pos,
        line: 1,
        column: pos as u32 + 1,
        len: 1,
    }
}

/// Aloca um [`NodeId`] de teste associado a um span sintético.
///
/// # Parâmetros
/// - `spans`: tabela lateral usada pelo programa parseado de teste;
/// - `pos`: posição sintética do nó.
///
/// # Retorno
/// - identificador alocado em `spans`.
fn node(spans: &mut NodeSpans, pos: usize) -> NodeId {
    spans.alloc(span(pos))
}

/// Interna um símbolo usado pela IR montada nos testes.
///
/// # Parâmetros
/// - `interner`: tabela compartilhada do cenário;
/// - `text`: lexema a ser internado.
///
/// # Retorno
/// - símbolo internado correspondente a `text`.
fn symbol(interner: &mut Interner, text: &str) -> Symbol {
    interner.entry(text).or_insert()
}

/// Constrói uma definição de rótulo para os testes.
///
/// # Parâmetros
/// - `name`: símbolo do rótulo;
/// - `node_id`: nó associado ao span do rótulo.
///
/// # Retorno
/// - [`LabelDef`] pronto para inserir em uma linha parseada.
fn label(name: Symbol, node_id: NodeId) -> LabelDef {
    LabelDef { name, node_id }
}

/// Constrói uma referência simbólica para operandos de teste.
///
/// # Parâmetros
/// - `name`: símbolo referenciado;
/// - `node_id`: nó associado ao span do operando.
///
/// # Retorno
/// - [`SymbolRef`] usado por [`AddressOperand`].
fn symbol_ref(name: Symbol, node_id: NodeId) -> SymbolRef {
    SymbolRef { name, node_id }
}

/// Constrói um literal numérico da IR para testes.
///
/// # Parâmetros
/// - `literal`: forma numérica sintática;
/// - `node_id`: nó associado ao span do literal.
///
/// # Retorno
/// - [`NumberLiteral`] usado por diretivas ou operandos com deslocamento.
fn number(literal: NumericLiteral, node_id: NodeId) -> NumberLiteral {
    NumberLiteral { literal, node_id }
}

/// Executa o montador de uma passagem em um programa de teste.
///
/// # Parâmetros
/// - `interner`: tabela usada para resolver literais numéricos;
/// - `program`: programa parseado construído pelo cenário.
///
/// # Retorno
/// - resultado produzido por [`OnePassAssembler::assemble`].
fn assemble(
    interner: &Interner,
    program: ParsedProgram,
) -> Result<AssemblyArtifacts, Vec<AssemblyError>> {
    let isa = InstructionSet::new();
    OnePassAssembler::new(interner, &isa).assemble(program)
}

/// Garante que a passagem única separa `.obj` resolvido de `.pen` pendente.
///
/// O cenário cobre referência de `TEXT` para rótulo definido em `DATA`,
/// deslocamento positivo, `CONST` assinado e reserva múltipla por `SPACE`.
#[test]
fn emits_resolved_object_and_unresolved_pending_program() {
    let mut interner = Interner::new();
    let loop_sym = symbol(&mut interner, "LOOP");
    let value_sym = symbol(&mut interner, "VALUE");
    let buffer_sym = symbol(&mut interner, "BUFFER");
    let one_sym = symbol(&mut interner, "1");
    let two_sym = symbol(&mut interner, "2");

    let mut spans = NodeSpans::new();
    let loop_label = node(&mut spans, 0);
    let load_node = node(&mut spans, 1);
    let load_operand = node(&mut spans, 2);
    let add_node = node(&mut spans, 3);
    let add_operand = node(&mut spans, 4);
    let add_offset = node(&mut spans, 5);
    let stop_node = node(&mut spans, 6);
    let value_label = node(&mut spans, 7);
    let value_const = node(&mut spans, 8);
    let value_literal = node(&mut spans, 9);
    let buffer_label = node(&mut spans, 10);
    let buffer_space = node(&mut spans, 11);
    let buffer_amount = node(&mut spans, 12);

    let program = ParsedProgram {
        text: vec![
            ParsedLine {
                label: Some(label(loop_sym, loop_label)),
                body: Instruction::OneOperand {
                    mnemonic: Mnemonic::Load,
                    operand: AddressOperand::Direct(symbol_ref(value_sym, load_operand)),
                    node_id: load_node,
                },
                node_id: load_node,
            },
            ParsedLine {
                label: None,
                body: Instruction::OneOperand {
                    mnemonic: Mnemonic::Add,
                    operand: AddressOperand::Offset {
                        base: symbol_ref(value_sym, add_operand),
                        offset: number(NumericLiteral::unsigned(one_sym), add_offset),
                        node_id: add_operand,
                    },
                    node_id: add_node,
                },
                node_id: add_node,
            },
            ParsedLine {
                label: None,
                body: Instruction::NoOperand {
                    mnemonic: Mnemonic::Stop,
                    node_id: stop_node,
                },
                node_id: stop_node,
            },
        ],
        data: vec![
            ParsedLine {
                label: Some(label(value_sym, value_label)),
                body: DataDirective::Const {
                    value: number(
                        NumericLiteral::signed(NumberSign::Minus, one_sym),
                        value_literal,
                    ),
                    node_id: value_const,
                },
                node_id: value_const,
            },
            ParsedLine {
                label: Some(label(buffer_sym, buffer_label)),
                body: DataDirective::Space {
                    amount: Some(number(NumericLiteral::unsigned(two_sym), buffer_amount)),
                    node_id: buffer_space,
                },
                node_id: buffer_space,
            },
        ],
        node_spans: spans,
    };

    let artifacts = assemble(&interner, program).expect("assembly should succeed");

    assert_eq!(
        artifacts.object.words,
        vec![10, 5, 1, 6, 14, u16::MAX, 0, 0]
    );
    assert_eq!(
        artifacts.pending.words,
        vec![10, 0, 1, 1, 14, u16::MAX, 0, 0]
    );
}

/// Garante que a passagem acumula símbolos indefinidos e rótulos duplicados.
#[test]
fn reports_undefined_symbols_and_duplicate_labels() {
    let mut interner = Interner::new();
    let missing_sym = symbol(&mut interner, "MISSING");
    let dup_sym = symbol(&mut interner, "DUP");
    let zero_sym = symbol(&mut interner, "0");

    let mut spans = NodeSpans::new();
    let load_node = node(&mut spans, 0);
    let load_operand = node(&mut spans, 1);
    let dup_text_label = node(&mut spans, 2);
    let stop_node = node(&mut spans, 3);
    let dup_data_label = node(&mut spans, 4);
    let const_node = node(&mut spans, 5);
    let const_value = node(&mut spans, 6);

    let program = ParsedProgram {
        text: vec![
            ParsedLine {
                label: None,
                body: Instruction::OneOperand {
                    mnemonic: Mnemonic::Load,
                    operand: AddressOperand::Direct(symbol_ref(missing_sym, load_operand)),
                    node_id: load_node,
                },
                node_id: load_node,
            },
            ParsedLine {
                label: Some(label(dup_sym, dup_text_label)),
                body: Instruction::NoOperand {
                    mnemonic: Mnemonic::Stop,
                    node_id: stop_node,
                },
                node_id: stop_node,
            },
        ],
        data: vec![ParsedLine {
            label: Some(label(dup_sym, dup_data_label)),
            body: DataDirective::Const {
                value: number(NumericLiteral::unsigned(zero_sym), const_value),
                node_id: const_node,
            },
            node_id: const_node,
        }],
        node_spans: spans,
    };

    let errors = match assemble(&interner, program) {
        Ok(_) => panic!("assembly should fail"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|err| {
        matches!(
            err.kind,
            AssemblyErrorKind::UndefinedSymbol { symbol } if symbol == missing_sym
        )
    }));
    assert!(errors.iter().any(|err| {
        matches!(
            err.kind,
            AssemblyErrorKind::SymbolAlreadyDefined { symbol, .. } if symbol == dup_sym
        )
    }));
}
