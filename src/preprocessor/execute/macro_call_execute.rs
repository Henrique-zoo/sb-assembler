use crate::{
    errors::PreprocessorError,
    interner::{Interner, Symbol},
    lexer::Token,
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Executa o fluxo completo de chamada de macro:
    /// detecção -> parse -> expansão.
    ///
    /// Retorna:
    /// - `Some(lines)` quando a linha é tratada como chamada de macro;
    /// - `None` quando não é chamada de macro.
    pub(in crate::preprocessor) fn execute_macro_call(
        &mut self,
        line: &[Token],
        interner: &mut Interner,
        errors: &mut Vec<PreprocessorError>,
    ) -> Option<Vec<Vec<Token>>> {
        let _ = (line, interner, errors);
        todo!()
    }

    /// Expande uma chamada já parseada para linhas concretas de tokens.
    fn expand_parsed_macro_call(
        &mut self,
        macro_name: Symbol,
        args: &[Symbol],
        call_line: &[Token],
    ) -> Result<Vec<Vec<Token>>, PreprocessorError> {
        let _ = (macro_name, args, call_line);
        todo!()
    }

    /// Faz substituição de parâmetros formais por argumentos reais no body.
    fn substitute_macro_call_args(
        &self,
        macro_body: &[Vec<Token>],
        args: &[Symbol],
    ) -> Result<Vec<Vec<Token>>, PreprocessorError> {
        let _ = (macro_body, args);
        todo!()
    }
}
