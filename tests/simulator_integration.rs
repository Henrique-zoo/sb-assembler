#![allow(dead_code, unused_imports, unused_variables)]

mod common;

mod assembler {
    pub type SignedWord = i16;
    pub type Word = u16;
}
#[path = "../src/assembly/mod.rs"]
mod assembly;
#[path = "../src/errors/mod.rs"]
mod errors;
#[path = "../src/interner/mod.rs"]
mod interner;
#[path = "../src/language/mod.rs"]
mod language;
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[path = "../src/parser/ir.rs"]
pub(crate) mod parser_ir;
#[path = "../src/parser/types.rs"]
pub(crate) mod parser_types;

mod parser {
    pub(crate) use crate::parser_ir as ir;
    pub(crate) use crate::parser_types as types;
}

use std::fs;

use assembler::Word;
use assembly::one_pass::{ObjectProgram, OnePassAssembler};
use common::{cleanup, temp_base_path};
use interner::{Interner, Symbol};
use language::{
    instructions::{InstructionSet, Mnemonic},
    numeric_literals::{NumberSign, NumericLiteral},
};
use lexer::Span;
use parser::{
    ir::{AddressOperand, DataDirective, Instruction, LabelDef, NumberLiteral, SymbolRef},
    types::{DataLine, NodeSpans, ParsedLine, ParsedProgram, TextLine},
};
use sb_assembler::{errors::SimulationError, simulator::simulate_obj_file};

fn span() -> Span {
    Span::default()
}

fn node(spans: &mut NodeSpans) -> parser::ir::NodeId {
    spans.alloc(span())
}

fn symbol(interner: &mut Interner, name: &str) -> Symbol {
    interner.entry(name).or_insert()
}

fn label(interner: &mut Interner, spans: &mut NodeSpans, name: &str) -> LabelDef {
    LabelDef {
        name: symbol(interner, name),
        node_id: node(spans),
    }
}

fn symbol_ref(interner: &mut Interner, spans: &mut NodeSpans, name: &str) -> SymbolRef {
    SymbolRef {
        name: symbol(interner, name),
        node_id: node(spans),
    }
}

fn number(interner: &mut Interner, spans: &mut NodeSpans, value: i16) -> NumberLiteral {
    let magnitude = value.unsigned_abs().to_string();
    let literal = if value < 0 {
        NumericLiteral::signed(NumberSign::Minus, symbol(interner, &magnitude))
    } else {
        NumericLiteral::unsigned(symbol(interner, &magnitude))
    };

    NumberLiteral {
        literal,
        node_id: node(spans),
    }
}

fn one_operand_line(
    interner: &mut Interner,
    spans: &mut NodeSpans,
    label: Option<&str>,
    mnemonic: Mnemonic,
    operand: &str,
) -> TextLine {
    let node_id = node(spans);

    ParsedLine {
        label: label.map(|name| self::label(interner, spans, name)),
        body: Instruction::OneOperand {
            mnemonic,
            operand: AddressOperand::Direct(symbol_ref(interner, spans, operand)),
            node_id,
        },
        node_id,
    }
}

fn no_operand_line(
    interner: &mut Interner,
    spans: &mut NodeSpans,
    label: Option<&str>,
    mnemonic: Mnemonic,
) -> TextLine {
    let node_id = node(spans);

    ParsedLine {
        label: label.map(|name| self::label(interner, spans, name)),
        body: Instruction::NoOperand { mnemonic, node_id },
        node_id,
    }
}

fn const_line(interner: &mut Interner, spans: &mut NodeSpans, name: &str, value: i16) -> DataLine {
    let node_id = node(spans);

    ParsedLine {
        label: Some(label(interner, spans, name)),
        body: DataDirective::Const {
            value: number(interner, spans, value),
            node_id,
        },
        node_id,
    }
}

fn space_line(interner: &mut Interner, spans: &mut NodeSpans, name: &str) -> DataLine {
    let node_id = node(spans);

    ParsedLine {
        label: Some(label(interner, spans, name)),
        body: DataDirective::Space {
            amount: None,
            node_id,
        },
        node_id,
    }
}

fn assemble_object(interner: &Interner, program: ParsedProgram) -> ObjectProgram {
    OnePassAssembler::new(interner, &InstructionSet::new())
        .assemble(program)
        .expect("programa de integração deve montar sem erros")
        .object
}

fn object_to_byte_text(object: &ObjectProgram) -> String {
    object
        .words
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .map(|byte| byte.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn simulate_object_via_file(
    object: &ObjectProgram,
    test_name: &str,
) -> Result<(), SimulationError> {
    let path = temp_base_path(test_name).with_extension("obj");
    fs::write(&path, object_to_byte_text(object))
        .expect("arquivo .obj temporário deve ser escrito");

    let result = simulate_obj_file(&path);

    cleanup(&[path]);
    result
}

#[test]
fn one_pass_object_round_trips_as_textual_bytes_and_executes_in_simulator() {
    let mut interner = Interner::new();
    let mut spans = NodeSpans::new();

    let program = ParsedProgram {
        text: vec![
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Load, "VALUE"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Add, "INCREMENT"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Store, "RESULT"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Load, "RESULT"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Sub, "EXPECTED"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Jmpz, "DONE"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Div, "ZERO"),
            no_operand_line(&mut interner, &mut spans, Some("DONE"), Mnemonic::Stop),
        ],
        data: vec![
            const_line(&mut interner, &mut spans, "VALUE", 7),
            const_line(&mut interner, &mut spans, "INCREMENT", 5),
            const_line(&mut interner, &mut spans, "EXPECTED", 12),
            const_line(&mut interner, &mut spans, "ZERO", 0),
            space_line(&mut interner, &mut spans, "RESULT"),
        ],
        node_spans: spans,
    };

    let object = assemble_object(&interner, program);

    assert_eq!(
        object.words,
        vec![
            10, 15, 1, 16, 11, 19, 10, 19, 2, 17, 8, 14, 4, 18, 14, 7, 5, 12, 0, 0
        ]
    );

    simulate_object_via_file(&object, "roundtrip")
        .expect("programa montado deve passar pelo .obj textual e executar até STOP");
}

#[test]
fn forward_text_label_from_assembler_controls_signed_jump_in_simulator() {
    let mut interner = Interner::new();
    let mut spans = NodeSpans::new();

    let program = ParsedProgram {
        text: vec![
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Load, "NEG"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Jmpn, "NEG_PATH"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Div, "ZERO"),
            one_operand_line(
                &mut interner,
                &mut spans,
                Some("NEG_PATH"),
                Mnemonic::Load,
                "ONE",
            ),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Store, "RESULT"),
            no_operand_line(&mut interner, &mut spans, None, Mnemonic::Stop),
        ],
        data: vec![
            const_line(&mut interner, &mut spans, "NEG", -1),
            const_line(&mut interner, &mut spans, "ZERO", 0),
            const_line(&mut interner, &mut spans, "ONE", 1),
            space_line(&mut interner, &mut spans, "RESULT"),
        ],
        node_spans: spans,
    };

    let object = assemble_object(&interner, program);

    assert_eq!(
        object.words,
        vec![10, 11, 6, 6, 4, 12, 10, 13, 11, 14, 14, Word::MAX, 0, 1, 0]
    );

    simulate_object_via_file(&object, "signed-forward-jump")
        .expect("JMPN deve seguir o rótulo forward resolvido");
}

#[test]
fn simulate_obj_file_runs_assembled_textual_byte_object_through_public_facade() {
    let mut interner = Interner::new();
    let mut spans = NodeSpans::new();

    let program = ParsedProgram {
        text: vec![
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Load, "VALUE"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Jmpp, "DONE"),
            one_operand_line(&mut interner, &mut spans, None, Mnemonic::Div, "ZERO"),
            no_operand_line(&mut interner, &mut spans, Some("DONE"), Mnemonic::Stop),
        ],
        data: vec![
            const_line(&mut interner, &mut spans, "VALUE", 1),
            const_line(&mut interner, &mut spans, "ZERO", 0),
        ],
        node_spans: spans,
    };

    let object = assemble_object(&interner, program);

    simulate_object_via_file(&object, "public-facade")
        .expect("fachada pública deve ler, carregar e simular o .obj");
}

#[test]
fn simulate_obj_file_reports_misaligned_textual_byte_object() {
    let path = temp_base_path("misaligned").with_extension("obj");
    fs::write(&path, "14 0 1").expect("arquivo .obj temporário deve ser escrito");

    let result = simulate_obj_file(&path);

    cleanup(&[path]);

    assert!(matches!(
        result,
        Err(SimulationError::MisalignedObjectFile { bytes: 3 })
    ));
}

#[test]
fn simulate_obj_file_reports_invalid_textual_byte() {
    let path = temp_base_path("invalid-byte").with_extension("obj");
    fs::write(&path, "14 0 256").expect("arquivo .obj temporário deve ser escrito");

    let result = simulate_obj_file(&path);

    cleanup(&[path]);

    assert!(matches!(
        result,
        Err(SimulationError::InvalidObjectByte { token }) if token == "256"
    ));
}
