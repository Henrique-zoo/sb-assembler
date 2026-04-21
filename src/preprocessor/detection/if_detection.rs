use crate::{
    lexer::{Token, TokenKind},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Indica se a linha parece uma diretiva `IF`.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// IF <expressão>
    /// ```
    ///
    /// Critério de triagem:
    /// - o prefixo deve ser `IF`;
    /// - aceita qualquer sufixo após o prefixo reconhecido.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let if_kw = preprocessor.keywords.if_kw;
    /// let cond = interner.entry("FLAG").or_insert();
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_if_line(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    ///     Token::new(TokenKind::Ident(cond), span),
    /// ]));
    ///
    /// // Caso permissivo (somente keyword)
    /// assert!(preprocessor.looks_like_if_line(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    /// ]));
    ///
    /// // Caso permissivo (`:` opcional)
    /// assert!(preprocessor.looks_like_if_line(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    ///     Token::new(TokenKind::Colon, span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_if_line(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    ///     Token::new(TokenKind::Ident(cond), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_if_line(&[
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    /// ```
    pub(in crate::preprocessor) fn looks_like_if_line(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == self.keywords.if_kw
        )
    }
}
