use crate::{lexer::Token, preprocessor::Preprocessor};

impl Preprocessor {
    /// Indica se a linha parece uma tentativa de chamada de macro.
    ///
    /// Heurística intencionalmente permissiva: basta a linha sinalizar intenção
    /// de chamada para o parser conseguir produzir diagnóstico sintático
    /// específico em caso de erro.
    pub(in crate::preprocessor) fn looks_like_macro_call(&self, line: &[Token]) -> bool {
        let _ = line;
        todo!()
    }

    /// Indica se a tentativa de chamada parece trazer lista de argumentos.
    ///
    /// Esta função existe para separar triagem de chamada e triagem de formato
    /// de argumentos, mantendo os detectores pequenos e especializados.
    pub(in crate::preprocessor) fn looks_like_macro_call_args(&self, line: &[Token]) -> bool {
        let _ = line;
        todo!()
    }
}
