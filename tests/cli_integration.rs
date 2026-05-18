mod common;

use std::fs;

use common::{cleanup, read, run_binary, temp_base_path};

#[test]
fn cli_simulates_obj_file_without_assembler_facade() {
    let path = temp_base_path("cli-output").with_extension("obj");
    fs::write(&path, "13 0 3 0 14 0 42 0").expect("arquivo .obj temporário deve ser escrito");

    let output = run_binary(&path);

    assert!(
        output.status.success(),
        "simulação via CLI falhou: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");

    cleanup(&[path]);
}

#[test]
fn cli_reports_lexer_error_and_does_not_write_pre_file() {
    let path = temp_base_path("invalid-asm").with_extension("asm");
    let output_path = path.with_extension("pre");
    fs::write(&path, "SECTION TEXT\n@\n").expect("arquivo .asm temporário deve ser escrito");

    let output = run_binary(&path);

    assert!(!output.status.success(), "arquivo inválido deve falhar");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Erro léxico"),
        "stderr deve exibir o erro real: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output_path.exists(),
        "não deve gerar .pre quando o pipeline falha"
    );

    cleanup(&[path, output_path]);
}

#[test]
fn cli_reports_assembly_error_and_does_not_write_obj_or_pen_files() {
    let path = temp_base_path("invalid-pre").with_extension("pre");
    let obj_path = path.with_extension("obj");
    let pen_path = path.with_extension("pen");
    fs::write(&path, "SECTION TEXT\nLOAD MISSING\nSTOP\n")
        .expect("arquivo .pre temporário deve ser escrito");

    let output = run_binary(&path);

    assert!(!output.status.success(), "montagem inválida deve falhar");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Erro de montagem"),
        "stderr deve exibir o erro real: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !obj_path.exists(),
        "não deve gerar .obj quando a montagem falha"
    );
    assert!(
        !pen_path.exists(),
        "não deve gerar .pen quando a montagem falha"
    );

    cleanup(&[path, obj_path, pen_path]);
}

#[test]
fn cli_runs_full_assignment_pipeline_across_all_three_modes() {
    let base = temp_base_path("full-cli");
    let asm_path = base.with_extension("asm");
    let pre_path = base.with_extension("pre");
    let obj_path = base.with_extension("obj");
    let pen_path = base.with_extension("pen");

    fs::write(
        &asm_path,
        "section data\nvalue: const 42\nsection text\noutput value\nstop\n",
    )
    .expect("arquivo .asm temporário deve ser escrito");

    let asm_output = run_binary(&asm_path);

    assert!(
        asm_output.status.success(),
        "modo .asm falhou: {}",
        String::from_utf8_lossy(&asm_output.stderr)
    );
    assert_eq!(
        read(&pre_path),
        "SECTION TEXT\nOUTPUT VALUE\nSTOP\nSECTION DATA\nVALUE: CONST 42"
    );

    let pre_output = run_binary(&pre_path);

    assert!(
        pre_output.status.success(),
        "modo .pre falhou: {}",
        String::from_utf8_lossy(&pre_output.stderr)
    );
    assert_eq!(read(&obj_path), "13 0 3 0 14 0 42 0");
    assert_eq!(read(&pen_path), "13 0 14 42");

    let obj_output = run_binary(&obj_path);

    assert!(
        obj_output.status.success(),
        "modo .obj falhou: {}",
        String::from_utf8_lossy(&obj_output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&obj_output.stdout), "42\n");

    cleanup(&[asm_path, pre_path, obj_path, pen_path]);
}
