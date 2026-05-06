//! Execução semântica das diretivas de seção.
//!
//! Este módulo aplica o efeito de `SECTION TEXT`/`SECTION DATA` após o parsing
//! sintático:
//! - atualiza `current_section`;
//! - emite a própria linha de seção no output lógico.

use crate::{
    lexer::Token,
    preprocessor::{LogicalLine, Preprocessor, ir::SectionDecl},
};

impl Preprocessor {
    /// Aplica os efeitos de uma diretiva `SECTION` já parseada.
    ///
    /// Efeitos:
    /// - atualiza `self.current_section` com a seção declarada;
    /// - retorna a própria linha de seção (`content` + `terminator`) para o
    ///   chamador anexar ao output.
    ///
    /// Contrato:
    /// - `line` deve ser a linha original que gerou `section_decl`;
    /// - `terminator` deve ser o terminador original dessa linha.
    ///
    /// Erros:
    /// - nenhum. A função assume que a validação sintática já ocorreu.
    pub(in crate::preprocessor) fn execute_section_directive(
        &mut self,
        section_decl: SectionDecl,
        line: &[Token],
        terminator: Token,
    ) -> Vec<LogicalLine> {
        self.current_section = section_decl.section;
        vec![LogicalLine {
            content: line.to_vec(),
            terminator,
        }]
    }
}
