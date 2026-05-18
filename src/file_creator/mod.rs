use std::fs;

use crate::{
    assembler::Word,
    interner::Interner,
    preprocessor::{PreprocessedProgram, PreprocessorRenderer},
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

    pub(crate) fn create_assembled_output_file(file_name: &str, words: &[Word]) {
        let output = words
            .iter()
            .map(|word| word.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(file_name, output)
            .expect("Falhou ao escrever o arquivo de saída da passagem única");
    }
}
