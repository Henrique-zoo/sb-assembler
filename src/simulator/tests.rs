use super::*;

fn object(words: Vec<Word>) -> ObjectProgram {
    ObjectProgram { words }
}

fn loaded_processor(words: Vec<Word>) -> Processor {
    let mut processor = Processor::new();
    processor.load(object(words)).unwrap();
    processor
}

fn run_program(words: Vec<Word>) -> Processor {
    let mut processor = loaded_processor(words);
    processor.run().unwrap();
    processor
}

#[test]
fn parses_object_words_from_textual_byte_source() {
    let program = object_program_from_obj_text("10 0 7 0 14 0 255 255").unwrap();

    assert_eq!(program.words, vec![10, 7, 14, Word::MAX]);
}

#[test]
fn rejects_textual_object_with_incomplete_word() {
    let err = match object_program_from_obj_text("10 0 7") {
        Ok(_) => panic!("misaligned object should fail"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        SimulationError::MisalignedObjectFile { bytes: 3 }
    ));
}

#[test]
fn rejects_textual_object_with_invalid_byte() {
    let err = match object_program_from_obj_text("10 0 256") {
        Ok(_) => panic!("invalid byte should fail"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        SimulationError::InvalidObjectByte { token } if token == "256"
    ));
}

#[test]
fn load_rejects_program_larger_than_memory() {
    let mut processor = Processor::new();

    let result = processor.load(object(vec![0; 65_537]));

    assert!(matches!(result, Err(SimulationError::ExceededMemorySize)));
}

#[test]
fn load_resets_registers_and_memory_before_copying_program() {
    let mut processor = loaded_processor(vec![14, 123]);
    processor.acc = 42;
    processor.pc = 7;
    processor.memory[10] = 99;

    processor.load(object(vec![14])).unwrap();

    assert_eq!(processor.acc, 0);
    assert_eq!(processor.pc, 0);
    assert_eq!(processor.memory[0], 14);
    assert_eq!(processor.memory[1], 0);
    assert_eq!(processor.memory[10], 0);
}

#[test]
fn runs_loaded_program_until_stop() {
    let processor = run_program(vec![10, 7, 1, 8, 11, 9, 14, 2, 3, 0]);

    assert_eq!(processor.acc, 5);
    assert_eq!(processor.memory[9], 5);
}

#[test]
fn runs_sub_mul_and_div_with_address_operands() {
    let processor = run_program(vec![
        10, 11, 2, 12, 3, 13, 4, 14, 11, 15, 14, 20, 5, 3, 5, 0,
    ]);

    assert_eq!(processor.acc, 9);
    assert_eq!(processor.memory[15], 9);
    assert_eq!(processor.pc, 10);
}

#[test]
fn copy_uses_first_operand_as_source_and_second_as_destination() {
    let processor = run_program(vec![9, 6, 7, 14, 0, 0, 42, 0]);

    assert_eq!(processor.memory[7], 42);
}

#[test]
fn arithmetic_wraps_when_accumulator_overflows() {
    let processor = run_program(vec![10, 7, 1, 8, 11, 9, 14, 32_767, 1, 0]);

    assert_eq!(processor.acc, SignedWord::MIN);
    assert_eq!(processor.memory[9], SignedWord::MIN as Word);
}

#[test]
fn unconditional_jump_changes_program_counter() {
    let processor = run_program(vec![5, 4, 99, 99, 14]);

    assert_eq!(processor.pc, 4);
}

#[test]
fn conditional_jump_uses_signed_accumulator() {
    let processor = run_program(vec![10, 7, 6, 6, 99, 99, 14, Word::MAX]);

    assert_eq!(processor.acc, -1);
    assert_eq!(processor.pc, 6);
}

#[test]
fn invalid_opcode_reports_opcode_and_program_counter() {
    let mut processor = loaded_processor(vec![5, 3, 14, 99]);

    let err = processor.run().unwrap_err();

    assert!(matches!(
        err,
        SimulationError::InvalidOpcode { opcode: 99, pc: 3 }
    ));
}

#[test]
fn division_by_zero_reports_program_counter() {
    let mut processor = loaded_processor(vec![10, 5, 4, 6, 14, 10, 0]);

    let err = processor.run().unwrap_err();

    assert!(matches!(err, SimulationError::DivisionByZero { pc: 2 }));
}

#[test]
fn missing_operand_past_memory_end_is_reported() {
    let mut processor = Processor::new();
    processor.memory[65_535] = 1;
    processor.pc = 65_535;

    let err = processor.run().unwrap_err();

    assert!(matches!(
        err,
        SimulationError::ProgramCounterOutOfBounds { pc: 65_536 }
    ));
}
