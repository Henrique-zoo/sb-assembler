use std::fs;

use crate::{
    assembler::Word,
    assembler::interner::Interner,
    assembler::preprocessor::{PreprocessedProgram, PreprocessorRenderer},
};

pub(crate) struct FileCreator {}

impl FileCreator {
    pub(crate) fn create_preprocessed_output_file(
        file_name: &str,
        preprocessed_tokens: &PreprocessedProgram,
        interner: &Interner,
    ) {
        let renderer = PreprocessorRenderer::new(interner);
        let preprocessor_result = renderer.render_preprocessed_program(preprocessed_tokens);
        fs::write(format!("{}.pre", file_name), preprocessor_result)
            .expect("Falhou ao escrever o arquivo de saída do pré-processador");
    }

    pub(crate) fn create_pending_output_file(file_name: &str, words: &[Word]) {
        let output = words
            .iter()
            .map(|word| word.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(file_name, output)
            .expect("Falhou ao escrever o arquivo de saída da passagem única");
    }

    pub(crate) fn create_object_output_file(file_name: &str, words: &[Word]) {
        let output = words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(file_name, output).expect("Falhou ao escrever o arquivo objeto");
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::FileCreator;

    fn temp_output_path(extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema deve estar após UNIX_EPOCH")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "sb-assembler-file-creator-{}-{nanos}.{extension}",
            std::process::id()
        ))
    }

    #[test]
    fn writes_pending_file_as_space_separated_text_words() {
        let path = temp_output_path("pen");

        FileCreator::create_pending_output_file(path.to_str().unwrap(), &[10, 7, 14]);

        let content = fs::read_to_string(&path).expect("arquivo .pen deve ser lido como texto");
        let _ = fs::remove_file(&path);

        assert_eq!(content, "10 7 14");
    }

    #[test]
    fn writes_object_file_as_space_separated_little_endian_byte_strings() {
        let path = temp_output_path("obj");

        FileCreator::create_object_output_file(path.to_str().unwrap(), &[10, 7, 14, u16::MAX]);

        let content = fs::read_to_string(&path).expect("arquivo .obj deve ser lido como texto");
        let _ = fs::remove_file(&path);

        assert_eq!(content, "10 0 7 0 14 0 255 255");
    }
}
