use std::fs;

use crate::{
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
}
