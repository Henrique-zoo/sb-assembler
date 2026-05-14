use crate::{
    errors::{DirectiveKind, DirectiveSyntaxErrorKind, ExpectedToken, PreprocessorError},
    lexer::{Span, Token, TokenKind},
    preprocessor::{Preprocessor, ir::SectionDecl, types::Section},
};

impl Preprocessor<'_> {
    /// Faz o parsing de uma diretiva `SECTION <TEXT|DATA>`.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// SECTION <TEXT|DATA>
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
    /// - `Ok(SectionDecl { .. })` quando a linha está na forma canônica.
    ///
    /// Erros:
    /// - propaga erros de keyword ausente/inesperada para `SECTION` ou `TEXT|DATA`;
    /// - propaga `TrailingTokens` quando há qualquer token após `TEXT|DATA`.
    ///
    /// Contrato:
    /// - consome o prefixo `SECTION <TEXT|DATA>` inteiro;
    /// - rejeita qualquer sufixo remanescente;
    /// - aloca um `NodeId` cobrindo a diretiva completa;
    /// - não altera `current_section`. Esse efeito pertence a
    ///   [`Self::execute_section_directive`].
    pub(in crate::preprocessor) fn parse_section_line(
        &mut self,
        line: &[Token],
    ) -> Result<SectionDecl, PreprocessorError> {
        let text_symbol = self.language_symbols.sections.text;
        let data_symbol = self.language_symbols.sections.data;

        let (tail, fallback_span) = self.consume_keyword(
            line,
            DirectiveKind::Section,
            self.language_symbols.preprocessor.section,
            Span::default(),
        )?;

        let (section, tail) = tail.split_first().ok_or_else(|| {
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive: DirectiveKind::Section,
                    expected: ExpectedToken::Keyword(text_symbol),
                },
                fallback_span,
            )
        })?;

        let Token {
            kind: TokenKind::Ident(expected_section_kind),
            span: section_span,
        } = section
        else {
            return Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::Section,
                    expected: ExpectedToken::Keyword(text_symbol),
                },
                section.span,
            ));
        };

        let section = if *expected_section_kind == text_symbol {
            Section::Text
        } else if *expected_section_kind == data_symbol {
            Section::Data
        } else {
            return Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::Section,
                    expected: ExpectedToken::Keyword(text_symbol),
                },
                section.span,
            ));
        };

        self.ensure_no_trailing_tokens(tail, DirectiveKind::Section)?;
        let span = self.consumed_prefix_span(line, tail, fallback_span);
        let node_id = self.alloc_node_id(span);

        Ok(SectionDecl { node_id, section })
    }
}
