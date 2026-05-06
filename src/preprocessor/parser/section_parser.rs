use crate::{
    errors::{DirectiveKind, PreprocessorError},
    lexer::{Span, Token},
    preprocessor::{Preprocessor, ir::SectionDecl, types::Section},
};

impl Preprocessor {
    /// Faz o parsing de uma diretiva `SECTION TEXT`.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// SECTION TEXT
    /// ```
    ///
    /// Papel no pipeline:
    /// - valida apenas a sintaxe da troca de seção;
    /// - retorna uma [`SectionDecl`] para o estágio de execução atualizar
    ///   `current_section`;
    /// - não emite linha para a saída preprocessada. A diretiva `SECTION` é
    ///   consumida pelo pré-processador e serve apenas para particionar as linhas
    ///   seguintes entre [`crate::preprocessor::PreprocessedProgram::text`] e
    ///   [`crate::preprocessor::PreprocessedProgram::data`].
    ///
    /// Retorno:
    /// - `Ok(SectionDecl { section: Section::Text, .. })` quando a linha está na
    ///   forma canônica.
    ///
    /// Erros:
    /// - propaga erros de keyword ausente/inesperada para `SECTION` ou `TEXT`;
    /// - propaga `TrailingTokens` quando há qualquer token após `TEXT`.
    pub(in crate::preprocessor) fn parse_text_section_line(
        &mut self,
        line: &[Token],
    ) -> Result<SectionDecl, PreprocessorError> {
        self.parse_section_line(line, Section::Text)
    }

    /// Faz o parsing de uma diretiva `SECTION DATA`.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// SECTION DATA
    /// ```
    ///
    /// Papel no pipeline:
    /// - valida apenas a sintaxe da troca de seção;
    /// - retorna uma [`SectionDecl`] para o estágio de execução atualizar
    ///   `current_section`;
    /// - não emite linha para a saída preprocessada. A diretiva `SECTION` é
    ///   consumida pelo pré-processador e serve apenas para particionar as linhas
    ///   seguintes entre [`crate::preprocessor::PreprocessedProgram::text`] e
    ///   [`crate::preprocessor::PreprocessedProgram::data`].
    ///
    /// Retorno:
    /// - `Ok(SectionDecl { section: Section::Data, .. })` quando a linha está na
    ///   forma canônica.
    ///
    /// Erros:
    /// - propaga erros de keyword ausente/inesperada para `SECTION` ou `DATA`;
    /// - propaga `TrailingTokens` quando há qualquer token após `DATA`.
    pub(in crate::preprocessor) fn parse_data_section_line(
        &mut self,
        line: &[Token],
    ) -> Result<SectionDecl, PreprocessorError> {
        self.parse_section_line(line, Section::Data)
    }

    /// Parser base para diretivas de seção.
    ///
    /// Contrato:
    /// - consome o prefixo `SECTION <TEXT|DATA>` inteiro;
    /// - rejeita qualquer sufixo remanescente;
    /// - aloca um `NodeId` cobrindo a diretiva completa;
    /// - não altera `current_section`. Esse efeito pertence a
    ///   [`Self::execute_section_directive`].
    fn parse_section_line(
        &mut self,
        line: &[Token],
        section: Section,
    ) -> Result<SectionDecl, PreprocessorError> {
        let (tail, fallback_span) = self.consume_keyword(
            line,
            DirectiveKind::Section,
            self.keywords.section_kw,
            Span::default(),
        )?;

        let expected_section_kind = match section {
            Section::Data => self.keywords.data_kw,
            _ => self.keywords.text_kw,
        };

        let (tail, fallback_span) = self.consume_keyword(
            tail,
            DirectiveKind::Section,
            expected_section_kind,
            fallback_span,
        )?;

        self.ensure_no_trailing_tokens(tail, DirectiveKind::Section)?;
        let span = self.consumed_prefix_span(line, tail, fallback_span);
        let node_id = self.alloc_node_id(span);

        Ok(SectionDecl { node_id, section })
    }
}
