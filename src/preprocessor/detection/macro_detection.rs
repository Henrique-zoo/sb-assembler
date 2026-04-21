use crate::{
    lexer::{Token, TokenKind},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Indica se a linha parece iniciar uma definição de macro.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// <Label>: MACRO
    /// ```
    ///
    /// Critério de triagem:
    /// - aceita `<Ident>: MACRO` como prefixo;
    /// - aceita `<Ident> MACRO` como prefixo (`:` opcional);
    /// - aceita qualquer sufixo após o prefixo reconhecido.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let label = interner.entry("ROT").or_insert();
    /// let macro_kw = preprocessor.keywords.macro_kw;
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_macro_header(&[
    ///     Token::new(TokenKind::Ident(label), span),
    ///     Token::new(TokenKind::Colon, span),
    ///     Token::new(TokenKind::Ident(macro_kw), span),
    /// ]));
    ///
    /// // Caso permissivo (`:` opcional)
    /// assert!(preprocessor.looks_like_macro_header(&[
    ///     Token::new(TokenKind::Ident(label), span),
    ///     Token::new(TokenKind::Ident(macro_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_macro_header(&[
    ///     Token::new(TokenKind::Ident(label), span),
    ///     Token::new(TokenKind::Colon, span),
    ///     Token::new(TokenKind::Ident(macro_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_macro_header(&[
    ///     Token::new(TokenKind::Ident(label), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    /// ```
    pub(in crate::preprocessor) fn looks_like_macro_header(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(_), ..},
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == self.keywords.macro_kw
        ) || matches!(
            line,
            [
                Token { kind: TokenKind::Ident(_), ..},
                Token { kind: TokenKind::Colon, .. },
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == self.keywords.macro_kw
        )
    }

    /// Indica se a linha parece encerrar uma definição de macro.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// ENDMACRO
    /// ```
    ///
    /// Critério de triagem:
    /// - o prefixo deve ser `ENDMACRO`;
    /// - aceita qualquer sufixo após o prefixo reconhecido.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let endmacro_kw = preprocessor.keywords.endmacro_kw;
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_endmacro_line(&[
    ///     Token::new(TokenKind::Ident(endmacro_kw), span),
    /// ]));
    ///
    /// // Caso permissivo (`:` opcional)
    /// assert!(preprocessor.looks_like_endmacro_line(&[
    ///     Token::new(TokenKind::Ident(endmacro_kw), span),
    ///     Token::new(TokenKind::Colon, span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_endmacro_line(&[
    ///     Token::new(TokenKind::Ident(endmacro_kw), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_endmacro_line(&[
    ///     Token::new(TokenKind::Ident(add), span),
    ///     Token::new(TokenKind::Ident(endmacro_kw), span),
    /// ]));
    /// ```
    pub(in crate::preprocessor) fn looks_like_endmacro_line(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [Token { kind: TokenKind::Ident(sym), .. }, ..]
            if *sym == self.keywords.endmacro_kw
        )
    }
}
