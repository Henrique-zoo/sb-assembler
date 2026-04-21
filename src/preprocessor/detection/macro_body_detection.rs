use crate::{lexer::Token, preprocessor::Preprocessor};

impl Preprocessor {
    /// Indica se uma linha, no contexto de definição de macro, tenta encerrar o
    /// corpo com `ENDMACRO`.
    ///
    /// Esta detecção é heurística/permissiva: o objetivo é encaminhar a linha
    /// para parsing e permitir erro específico quando houver tentativa inválida.
    pub(in crate::preprocessor) fn looks_like_macro_body_terminator(
        &self,
        line: &[Token],
    ) -> bool {
        let _ = line;
        todo!()
    }

    /// Indica se uma linha pode ser tratada como conteúdo de body de macro.
    ///
    /// Em geral, toda linha não vazia dentro de `State::DefiningMacro` é
    /// candidata a body, salvo quando detectada como terminador.
    pub(in crate::preprocessor) fn looks_like_macro_body_content(
        &self,
        line: &[Token],
    ) -> bool {
        let _ = line;
        todo!()
    }
}
