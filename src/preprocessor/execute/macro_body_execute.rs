use crate::{errors::PreprocessorError, lexer::Token, preprocessor::Preprocessor};

impl Preprocessor {
    /// Executa um passo de processamento de body de macro.
    ///
    /// Espera ser chamado no estado `DefiningMacro` para decidir entre:
    /// - encerrar definição (`ENDMACRO`);
    /// - acumular linha como conteúdo do body.
    pub(in crate::preprocessor) fn execute_macro_body_step(
        &mut self,
        line: &[Token],
    ) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Acumula uma linha validada no corpo da macro em construção.
    fn push_macro_body_content(&mut self, line: &[Token]) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Executa o encerramento semântico de uma definição de macro.
    fn execute_macro_body_terminator(&mut self) -> Result<(), PreprocessorError> {
        todo!()
    }
}
