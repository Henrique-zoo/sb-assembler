use crate::{
    lexer::Token,
    preprocessor::{Preprocessor, TokenKind},
};

impl Preprocessor {
    /// Indica se a linha parece uma tentativa de chamada de macro.
    ///
    /// Forma canônica da linguagem:
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// ```
    ///
    /// Critério de triagem (heurístico):
    /// - o único critério é: o primeiro token da linha deve ser `Ident(sym)`;
    /// - esse `sym` não pode ser keyword;
    /// - ignora o restante da linha nesta etapa.
    ///
    /// Motivação:
    /// - a detecção aqui é intencionalmente permissiva para não perder
    ///   tentativas de chamada malformadas;
    /// - a validação estrita da sintaxe dos argumentos pertence ao parser de
    ///   macro call (`parse_macro_call` / `parse_macro_call_args`), que consegue
    ///   emitir diagnósticos específicos.
    ///
    /// Limites de responsabilidade:
    /// - **não** valida a lista de argumentos;
    /// - **não** executa expansão.
    ///
    /// Em outras palavras, este detector só responde:
    /// "essa linha começa com identificador não-keyword?".
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let rot = interner.entry("ROT").or_insert();
    /// let r1 = interner.entry("R1").or_insert();
    ///
    /// // Candidata canônica (nome + args)
    /// assert!(preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(rot), span),
    ///     Token::new(TokenKind::Ident(r1), span),
    /// ]));
    ///
    /// // Candidata mínima (somente nome)
    /// assert!(preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(rot), span),
    /// ]));
    ///
    /// // Não candidata: começa com pontuação
    /// assert!(!preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Comma, span),
    /// ]));
    ///
    /// // Não candidata: começa com keyword
    /// let if_kw = preprocessor.keywords.if_kw;
    /// assert!(!preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    /// ]));
    ///
    /// ```
    pub(in crate::preprocessor) fn looks_like_macro_call(&self, line: &[Token]) -> bool {
        if let Some(Token {
            kind: TokenKind::Ident(sym),
            ..
        }) = line.first()
        {
            !self.keyword_table.is_reserved(*sym)
        } else {
            false
        }
    }
}
