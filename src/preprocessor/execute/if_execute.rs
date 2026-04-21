use crate::{errors::PreprocessorError, lexer::Token, preprocessor::Preprocessor};

impl Preprocessor {
    /// Processa uma linha candidata à diretiva `IF`.
    pub(in crate::preprocessor) fn process_if(
        &mut self,
        line: &[Token],
        output: &mut Vec<Token>,
    ) -> Result<(), PreprocessorError> {
        let _ = (line, output);
        todo!()
    }
}
