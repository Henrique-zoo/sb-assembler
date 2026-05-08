//! Detectores de diretivas de seção (`SECTION TEXT` e `SECTION DATA`).
//!
//! Este arquivo concentra a triagem heurística das diretivas de seção usadas
//! pelo pré-processador para identificar trocas entre `TEXT` e `DATA`.
//!
//! Assim como os demais `looks_like_*`, as funções aqui são permissivas:
//! - reconhecem intenção de uso da diretiva pelo prefixo;
//! - deixam validação sintática para `parser`;
//! - deixam a atualização de `current_section` para `execute`.

use crate::{
    interner::Symbol,
    lexer::{Token, TokenKind},
    preprocessor::Preprocessor,
};

impl Preprocessor<'_> {
    /// Indica se a linha parece declarar ou alternar para uma seção.
    ///
    /// Esta função faz a triagem comum para `SECTION TEXT` e `SECTION DATA`.
    /// A validação sintática estrita continua no parser; aqui a intenção é só
    /// identificar linhas candidatas sem perder casos malformados.
    ///
    /// Critério de triagem:
    /// - aceita forma canônica `SECTION <KIND> ...`;
    /// - aceita `<KIND> ...` para capturar ausência de `SECTION`;
    /// - aceita `<IdentQualquer> <KIND> ...` para capturar typo em `SECTION`.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita;
    /// - é usada pelos wrappers específicos de `TEXT` e `DATA`.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let section_kw = preprocessor.language_symbols.preprocessor.section;
    /// let text_kw = preprocessor.language_symbols.sections.text;
    /// let data_kw = preprocessor.language_symbols.sections.data;
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_section_kind_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(text_kw), span),
    /// ], text_kw));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_section_kind_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(data_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ], data_kw));
    ///
    /// // Caso permissivo sem `SECTION`
    /// assert!(preprocessor.looks_like_section_kind_line(&[
    ///     Token::new(TokenKind::Ident(text_kw), span),
    /// ], text_kw));
    ///
    /// // Caso permissivo com possível typo em `SECTION`
    /// let secao_kw = interner.entry("SECAO").or_insert();
    /// assert!(preprocessor.looks_like_section_kind_line(&[
    ///     Token::new(TokenKind::Ident(secao_kw), span),
    ///     Token::new(TokenKind::Ident(data_kw), span),
    /// ], data_kw));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_section_kind_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ], text_kw));
    /// ```
    pub(in crate::preprocessor) fn looks_like_section_line(&self, line: &[Token]) -> bool {
        let section_symbols = &self.language_symbols.sections;
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(sym1), ..},
                Token { kind: TokenKind::Ident(sym2), .. },
                ..
            ] if *sym2 == section_symbols.data || *sym2 == section_symbols.text
        ) || matches!(
            line,
            [
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == section_symbols.data || *sym == section_symbols.text
        )
    }
}
