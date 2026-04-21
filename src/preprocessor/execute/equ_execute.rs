use crate::{errors::PreprocessorError, lexer::Token, preprocessor::Preprocessor};

impl Preprocessor {
    /// Processa uma linha candidata à diretiva `EQU`.
    pub(in crate::preprocessor) fn process_equ(
        &mut self,
        line: &[Token],
    ) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }
}
