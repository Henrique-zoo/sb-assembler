use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn temp_base_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("relógio do sistema deve estar após UNIX_EPOCH")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "sb-assembler-{test_name}-{}-{nanos}",
        std::process::id()
    ))
}

pub fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("arquivo de saída deve existir")
}

pub fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[allow(dead_code)]
pub fn run_binary(path: impl AsRef<Path>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sb-assembler"))
        .arg(path.as_ref())
        .output()
        .expect("binário deve executar")
}
