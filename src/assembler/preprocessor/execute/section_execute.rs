//! Execução semântica das diretivas de seção.
//!
//! Este módulo aplica o efeito de `SECTION TEXT`/`SECTION DATA` após o parsing
//! sintático:
//! - atualiza `current_section`;
//! - deixa a linha de `SECTION` fora da saída preprocessada.

use crate::assembler::preprocessor::{Preprocessor, ir::SectionDecl};

impl Preprocessor<'_> {
    /// Aplica os efeitos de uma diretiva `SECTION` já parseada.
    ///
    /// Efeitos:
    /// - atualiza `self.current_section` com a seção declarada;
    /// - não retorna linha emitida, porque `SECTION` é uma diretiva de controle
    ///   do pré-processador e não parte do programa repassado ao parser.
    ///
    /// Erros:
    /// - nenhum. A função assume que a validação sintática já ocorreu.
    pub(in crate::assembler::preprocessor) fn execute_section_directive(
        &mut self,
        section_decl: SectionDecl,
    ) {
        self.current_section = section_decl.section;
    }
}
