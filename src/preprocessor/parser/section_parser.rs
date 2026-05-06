use crate::{
    errors::{DirectiveKind, PreprocessorError},
    lexer::{Span, Token},
    preprocessor::{Preprocessor, ir::SectionDecl, types::Section},
};

impl Preprocessor {
    /// Faz o parsing de uma linha candidata à diretiva `SECTION TEXT`.
    ///
    /// Retorno:
    /// - `Ok(SectionDecl { section: Section::Text, .. })`.
    pub(in crate::preprocessor) fn parse_text_section_line(
        &mut self,
        line: &[Token],
    ) -> Result<SectionDecl, PreprocessorError> {
        self.parse_section_line(line, Section::Text)
    }

    /// Faz o parsing de uma linha candidata à diretiva `SECTION DATA`.
    ///
    /// Retorno:
    /// - `Ok(SectionDecl { section: Section::Data, .. })`.
    pub(in crate::preprocessor) fn parse_data_section_line(
        &mut self,
        line: &[Token],
    ) -> Result<SectionDecl, PreprocessorError> {
        self.parse_section_line(line, Section::Data)
    }

    /// Parser base para diretivas de seção.
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
