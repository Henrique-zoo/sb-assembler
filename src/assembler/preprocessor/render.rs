use crate::{
    assembler::interner::Interner,
    assembler::lexer::{Token, TokenKind},
    assembler::preprocessor::{LogicalLine, PreprocessedProgram},
};

pub struct PreprocessorRenderer<'a> {
    interner: &'a Interner,
}

impl<'a> PreprocessorRenderer<'a> {
    pub(crate) fn new(interner: &'a Interner) -> Self {
        Self { interner }
    }

    pub(crate) fn token_to_text(&self, token: &Token) -> String {
        match token.kind {
            TokenKind::Ident(sym) | TokenKind::Number(sym) => self
                .interner
                .get_str(sym)
                .expect("symbol should be interned")
                .to_owned(),
            TokenKind::Ampersand => "&".to_owned(),
            TokenKind::Comma => ",".to_owned(),
            TokenKind::Colon => ":".to_owned(),
            TokenKind::Plus => "+".to_owned(),
            TokenKind::Minus => "-".to_owned(),
            TokenKind::NewLine => "\\n".to_owned(),
        }
    }

    pub(crate) fn render_preprocessor_lines(&self, lines: &[LogicalLine]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.content
                    .iter()
                    .map(|token| self.token_to_text(token))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .map(|s| self.format_assembly_line(&s))
            .collect()
    }

    pub(crate) fn format_assembly_line(&self, line: &str) -> String {
        let step1 = line.replace(" :", ":");
        let step2 = step1
            .replace(" , ", ",")
            .replace(" ,", ",")
            .replace(", ", ",");
        let step3 = step2.replace("+ ", "+").replace("- ", "-");
        step3
    }

    pub(crate) fn render_preprocessor_output(&self, lines: &[LogicalLine]) -> String {
        self.render_preprocessor_lines(lines).join("\n")
    }

    pub(crate) fn render_preprocessed_section(
        &self,
        section_name: &str,
        lines: &[LogicalLine],
    ) -> String {
        if lines.is_empty() {
            String::new()
        } else {
            format!(
                "SECTION {section_name}\n{}",
                self.render_preprocessor_output(lines)
            )
        }
    }

    pub(crate) fn render_preprocessed_program(&self, program: &PreprocessedProgram) -> String {
        [
            self.render_preprocessed_section("TEXT", &program.text),
            self.render_preprocessed_section("DATA", &program.data),
        ]
        .into_iter()
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    }
}
