//! Detectores de diretivas de seção (`SECTION TEXT` e `SECTION DATA`).
//!
//! Este arquivo concentra a triagem heurística das diretivas de seção usadas
//! pelo pré-processador para identificar o contexto corrente do programa.
//!
//! Assim como os demais `looks_like_*`, as funções aqui são permissivas:
//! - reconhecem intenção de uso da diretiva pelo prefixo;
//! - deixam validação sintática/semântica detalhada para parser/execute.

use crate::{
    interner::Symbol,
    lexer::{Token, TokenKind},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Indica se a linha parece declarar/alternar para a seção `TEXT`.
    ///
    /// Critério de triagem:
    /// - aceita forma canônica `SECTION TEXT ...`;
    /// - aceita `TEXT ...` para capturar ausência de `SECTION`;
    /// - aceita `<IdentQualquer> TEXT ...` para capturar typo em `SECTION`.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let section_kw = preprocessor.keywords.section_kw;
    /// let text_kw = preprocessor.keywords.text_kw;
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_text_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(text_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_text_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(text_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    ///
    /// // Caso permissivo sem `SECTION`
    /// assert!(preprocessor.looks_like_text_section_line(&[
    ///     Token::new(TokenKind::Ident(text_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com possível typo em `SECTION`
    /// let secao_kw = interner.entry("SECAO").or_insert();
    /// assert!(preprocessor.looks_like_text_section_line(&[
    ///     Token::new(TokenKind::Ident(secao_kw), span),
    ///     Token::new(TokenKind::Ident(text_kw), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_text_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    /// ```
    pub(in crate::preprocessor) fn looks_like_text_section_line(&self, line: &[Token]) -> bool {
        self.looks_like_section_kind_line(line, self.keywords.text_kw)
    }

    /// Indica se a linha parece declarar/alternar para a seção `DATA`.
    ///
    /// Critério de triagem:
    /// - aceita forma canônica `SECTION DATA ...`;
    /// - aceita `DATA ...` para capturar ausência de `SECTION`;
    /// - aceita `<IdentQualquer> DATA ...` para capturar typo em `SECTION`.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let section_kw = preprocessor.keywords.section_kw;
    /// let data_kw = preprocessor.keywords.data_kw;
    /// let space = interner.entry("SPACE").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_data_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(data_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_data_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(data_kw), span),
    ///     Token::new(TokenKind::Ident(space), span),
    /// ]));
    ///
    /// // Caso permissivo sem `SECTION`
    /// assert!(preprocessor.looks_like_data_section_line(&[
    ///     Token::new(TokenKind::Ident(data_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com possível typo em `SECTION`
    /// let secao_kw = interner.entry("SECAO").or_insert();
    /// assert!(preprocessor.looks_like_data_section_line(&[
    ///     Token::new(TokenKind::Ident(secao_kw), span),
    ///     Token::new(TokenKind::Ident(data_kw), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_data_section_line(&[
    ///     Token::new(TokenKind::Ident(section_kw), span),
    ///     Token::new(TokenKind::Ident(space), span),
    /// ]));
    /// ```
    pub(in crate::preprocessor) fn looks_like_data_section_line(&self, line: &[Token]) -> bool {
        self.looks_like_section_kind_line(line, self.keywords.data_kw)
    }

    fn looks_like_section_kind_line(&self, line: &[Token], section_kind_kw: Symbol) -> bool {
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(sym1), ..},
                Token { kind: TokenKind::Ident(sym2), .. },
                ..
            ] if *sym2 == section_kind_kw
        ) || matches!(
            line,
            [
                Token { kind: TokenKind::Ident(sym1), .. },
                ..
            ] if *sym1 == section_kind_kw
        )
    }
}
