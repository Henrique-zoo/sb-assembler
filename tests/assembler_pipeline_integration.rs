mod common;

use sb_assembler::assembler::{Assembler, AssemblerError, AssemblerStage};

use common::{cleanup, read, temp_base_path};

fn assert_single_diagnostic(
    err: AssemblerError,
    stage: AssemblerStage,
    line: Option<u32>,
    column: Option<u32>,
    message_fragment: &str,
) {
    let diagnostics = err.diagnostics();

    assert_eq!(diagnostics.len(), 1, "diagnósticos: {err}");
    assert_eq!(diagnostics[0].stage(), stage);
    assert_eq!(diagnostics[0].line(), line);
    assert_eq!(diagnostics[0].column(), column);
    assert!(
        diagnostics[0].message().contains(message_fragment),
        "mensagem inesperada: {}",
        diagnostics[0].message()
    );
}

#[test]
fn assembler_generates_pre_file_from_asm_with_preprocessor_features() {
    let base = temp_base_path("asm-to-pre");
    let pre_path = base.with_extension("pre");
    let source = r#"
init: macro &dst
load &dst
endmacro

enable equ 1

section data
value: const enable

section text
if enable
load value
init value
if 0
store value
stop
"#;

    let result = Assembler::new(source).generate_preprocessed_file(base.to_str().unwrap());

    assert!(result.is_ok(), "pré-processamento deve passar: {result:?}");
    assert_eq!(
        read(&pre_path),
        "SECTION TEXT\nLOAD VALUE\nLOAD VALUE\nSTOP\nSECTION DATA\nVALUE: CONST 1"
    );

    cleanup(&[pre_path]);
}

#[test]
fn assembler_generates_obj_and_pen_from_pre_file() {
    let base = temp_base_path("pre-to-artifacts");
    let obj_path = base.with_extension("obj");
    let pen_path = base.with_extension("pen");
    let source = r#"
section text
output value
stop
section data
value: const 42
"#;

    let result = Assembler::new(source).generate_obj_and_pen_files(base.to_str().unwrap());

    assert!(result.is_ok(), "montagem deve passar: {result:?}");
    assert_eq!(read(&obj_path), "13 0 3 0 14 0 42 0");
    assert_eq!(read(&pen_path), "13 0 14 42");

    cleanup(&[obj_path, pen_path]);
}

#[test]
fn assembler_reports_lexer_error_and_does_not_write_pre_file() {
    let base = temp_base_path("lexer-error");
    let pre_path = base.with_extension("pre");

    let err = Assembler::new("SECTION TEXT\n@\n")
        .generate_preprocessed_file(base.to_str().unwrap())
        .expect_err("caractere inválido deve falhar no lexer");

    assert_single_diagnostic(
        err,
        AssemblerStage::Lexer,
        Some(2),
        Some(1),
        "caractere inválido",
    );
    assert!(!pre_path.exists(), "erro léxico não deve gerar .pre");

    cleanup(&[pre_path]);
}

#[test]
fn assembler_reports_preprocessor_error_and_does_not_write_pre_file() {
    let base = temp_base_path("preprocessor-error");
    let pre_path = base.with_extension("pre");

    let err = Assembler::new("ORPHAN\nSECTION TEXT\nSTOP\n")
        .generate_preprocessed_file(base.to_str().unwrap())
        .expect_err("linha fora de seção deve falhar no pré-processador");

    assert_single_diagnostic(
        err,
        AssemblerStage::Preprocessor,
        Some(1),
        Some(1),
        "linha fora de seção",
    );
    assert!(
        !pre_path.exists(),
        "erro de pré-processamento não deve gerar .pre"
    );

    cleanup(&[pre_path]);
}

#[test]
fn assembler_reports_unterminated_macro_when_new_macro_starts_inside_previous_one() {
    let base = temp_base_path("nested-macro-error");
    let pre_path = base.with_extension("pre");
    let source = concat!(
        "ACCUM: MACRO &DST, &SRC, &TMP\n",
        "LOAD &TMP\n",
        "ADD &TMP, &SRC\n",
        "STORE &DST\n",
        "\n",
        "CLEAR: MACRO &DST\n",
        "LOAD ZERO\n",
        "STORE &DST\n",
        "ENDMACRO\n",
        "\n",
        "ZERO EQU 0\n",
        "ENABLE_LOG EQU 1\n",
        "\n",
        "SECTION DATA\n",
        "INPUT SPACE\n",
        "AUX SPACE\n",
        "RESULT SPACE\n",
        "LOGPTR SPACE\n",
        "\n",
        "SECTION TEXT\n",
        "ACCUM RESULT, INPUT, AUX\n",
        "IF ENABLE_LOG\n",
        "STORE LOGPTR\n",
        "IF 0\n",
        "ADD INPUT, 1\n",
        "CLEAR AUX\n",
    );

    let err = Assembler::new(source)
        .generate_preprocessed_file(base.to_str().unwrap())
        .expect_err("macro anterior sem ENDMACRO deve falhar no pré-processador");
    let rendered = err.to_string();

    assert!(
        rendered.contains("CLEAR: MACRO &DST"),
        "diagnóstico deve apontar para a macro iniciada antes do ENDMACRO:\n{rendered}"
    );
    assert!(
        rendered.contains("definição de macro sem `ENDMACRO`"),
        "diagnóstico deve reportar macro não terminada:\n{rendered}"
    );
    assert!(
        !rendered.contains("CLEAR AUX"),
        "CLEAR deve ser processada como macro válida, não como macro indefinida:\n{rendered}"
    );
    assert!(
        !pre_path.exists(),
        "erro de pré-processamento não deve gerar .pre"
    );

    cleanup(&[pre_path]);
}

#[test]
fn assembler_reports_parser_error_and_does_not_write_artifacts() {
    let base = temp_base_path("parser-error");
    let obj_path = base.with_extension("obj");
    let pen_path = base.with_extension("pen");

    let err = Assembler::new("SECTION TEXT\nLOAD\nSECTION DATA\nVALUE: CONST 1\n")
        .generate_obj_and_pen_files(base.to_str().unwrap())
        .expect_err("instrução sem operando deve falhar no parser");

    assert_single_diagnostic(err, AssemblerStage::Parser, Some(2), Some(1), "seção TEXT");
    assert!(!obj_path.exists(), "erro de parser não deve gerar .obj");
    assert!(!pen_path.exists(), "erro de parser não deve gerar .pen");

    cleanup(&[obj_path, pen_path]);
}

#[test]
fn assembler_reports_assembly_error_and_does_not_write_artifacts() {
    let base = temp_base_path("assembly-error");
    let obj_path = base.with_extension("obj");
    let pen_path = base.with_extension("pen");

    let err = Assembler::new("SECTION TEXT\nLOAD MISSING\nSTOP\n")
        .generate_obj_and_pen_files(base.to_str().unwrap())
        .expect_err("símbolo indefinido deve falhar na montagem");

    let rendered = err.to_string();
    assert!(
        rendered.contains("2 | LOAD MISSING"),
        "diagnóstico deve exibir a linha com erro:\n{rendered}"
    );
    assert!(
        rendered.contains("|      ^^^^^^^ símbolo `MISSING` não foi definido"),
        "diagnóstico deve marcar o símbolo indefinido:\n{rendered}"
    );

    assert_single_diagnostic(err, AssemblerStage::Assembly, Some(2), Some(6), "MISSING");
    assert!(!obj_path.exists(), "erro de montagem não deve gerar .obj");
    assert!(!pen_path.exists(), "erro de montagem não deve gerar .pen");

    cleanup(&[obj_path, pen_path]);
}
