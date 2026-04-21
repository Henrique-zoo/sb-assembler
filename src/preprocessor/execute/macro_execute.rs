use crate::{
    errors::PreprocessorError,
    interner::Interner,
    lexer::Token,
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Aplica os efeitos de um cabeçalho de macro já validado pelo parser.
    pub(in crate::preprocessor) fn execute_macro_header(&mut self) {
        todo!()
    }

    /// Finaliza a definição de macro corrente e atualiza o estado/tabela.
    pub(in crate::preprocessor) fn finish_macro_definition(
        &mut self,
    ) -> Result<(), PreprocessorError> {
        todo!()
    }

    /// Tenta expandir uma chamada de macro em uma ou mais linhas de tokens.
    ///
    /// Retorna:
    /// - `Some(expanded_lines)` quando a linha representa chamada de macro;
    /// - `None` quando não é chamada de macro.
    pub(in crate::preprocessor) fn expand_macro_call(
        &mut self,
        line: &[Token],
        interner: &mut Interner,
        errors: &mut Vec<PreprocessorError>,
    ) -> Option<Vec<Vec<Token>>> {
        let _ = (line, interner, errors);
        todo!()
    }
}
