use crate::{
    assembler::lexer::{Token, TokenKind},
    assembler::preprocessor::Preprocessor,
};

impl Preprocessor<'_> {
    /// Indica se a linha parece uma tentativa de chamada de macro.
    ///
    /// Forma capturada por este detector:
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// ```
    ///
    /// Critério de triagem (heurístico):
    /// - a linha deve começar com `Ident(sym)`;
    /// - esse `sym` não pode ser keyword;
    /// - se houver um segundo token, ele não pode ser `:`.
    ///
    /// Motivação:
    /// - a detecção aqui é intencionalmente permissiva para não perder
    ///   tentativas de chamada malformadas;
    /// - o filtro contra `:` evita rotear linhas com prefixo de label
    ///   (`<Ident>:`) como chamadas de macro;
    /// - a validação estrita da sintaxe dos argumentos pertence ao parser de
    ///   macro call (`parse_macro_call` / `parse_macro_call_args`), que consegue
    ///   emitir diagnósticos específicos.
    ///
    /// Limites de responsabilidade:
    /// - **não** valida a lista de argumentos;
    /// - **não** verifica se a macro existe;
    /// - **não** executa expansão.
    ///
    /// Em outras palavras, este detector só responde:
    /// "essa linha começa com identificador não-keyword sem prefixo de label?".
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
    /// // Candidata malformada, mas roteável para o parser de chamadas
    /// assert!(preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(rot), span),
    ///     Token::new(TokenKind::Comma, span),
    /// ]));
    ///
    /// // Não candidata: começa com pontuação
    /// assert!(!preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Comma, span),
    /// ]));
    ///
    /// // Não candidata: começa com keyword
    /// let if_kw = preprocessor.language_symbols.preprocessor.if_;
    /// assert!(!preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(if_kw), span),
    /// ]));
    ///
    /// // Candidata mínima (somente nome)
    /// assert!(preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(rot), span),
    /// ]));
    ///
    /// // Não candidata: prefixo de label
    /// assert!(!preprocessor.looks_like_macro_call(&[
    ///     Token::new(TokenKind::Ident(rot), span),
    ///     Token::new(TokenKind::Colon, span),
    /// ]));
    ///
    /// ```
    pub(in crate::assembler::preprocessor) fn looks_like_macro_call(&self, line: &[Token]) -> bool {
        match line {
            [
                Token {
                    kind: TokenKind::Ident(sym),
                    ..
                },
                tail @ ..,
            ] if !self.language_symbols.is_reserved(*sym) => !matches!(
                tail.first(),
                Some(Token {
                    kind: TokenKind::Colon,
                    ..
                })
            ),
            _ => false,
        }
    }
}
