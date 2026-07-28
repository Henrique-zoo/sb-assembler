use crate::{
    assembler::lexer::{Token, TokenKind},
    assembler::preprocessor::Preprocessor,
};

impl Preprocessor<'_> {
    /// Indica se a linha parece uma diretiva `EQU`.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// <alias> EQU <number>
    /// ```
    ///
    /// Critério de triagem:
    /// - aceita `<alias> EQU` como prefixo;
    /// - aceita `<alias>: EQU` como prefixo (`:` opcional);
    /// - aceita qualquer sufixo após o prefixo reconhecido.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let span = Span { pos: 0, line: 1, column: 1, len: 1 };
    /// let alias = interner.entry("N").or_insert();
    /// let equ_kw = preprocessor.language_symbols.preprocessor.equ;
    /// let one = interner.entry("1").or_insert();
    /// let add = interner.entry("ADD").or_insert();
    ///
    /// // Caso canônico
    /// assert!(preprocessor.looks_like_equ_line(&[
    ///     Token::new(TokenKind::Ident(alias), span),
    ///     Token::new(TokenKind::Ident(equ_kw), span),
    ///     Token::new(TokenKind::Number(one), span),
    /// ]));
    ///
    /// // Caso permissivo (`:` opcional)
    /// assert!(preprocessor.looks_like_equ_line(&[
    ///     Token::new(TokenKind::Ident(alias), span),
    ///     Token::new(TokenKind::Colon, span),
    ///     Token::new(TokenKind::Ident(equ_kw), span),
    /// ]));
    ///
    /// // Caso permissivo com sufixo extra
    /// assert!(preprocessor.looks_like_equ_line(&[
    ///     Token::new(TokenKind::Ident(alias), span),
    ///     Token::new(TokenKind::Ident(equ_kw), span),
    ///     Token::new(TokenKind::Number(one), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    ///
    /// // Caso não candidato
    /// assert!(!preprocessor.looks_like_equ_line(&[
    ///     Token::new(TokenKind::Ident(alias), span),
    ///     Token::new(TokenKind::Ident(add), span),
    /// ]));
    /// ```
    pub(in crate::assembler::preprocessor) fn looks_like_equ_line(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(_), .. },
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == self.language_symbols.preprocessor.equ
        ) || matches!(
            line,
            [
                Token { kind: TokenKind::Ident(_), .. },
                Token { kind: TokenKind::Colon, .. },
                Token { kind: TokenKind::Ident(sym), .. },
                ..
            ] if *sym == self.language_symbols.preprocessor.equ
        )
    }

    /// Indica se a linha parece usar um alias `EQU` como operando de `DATA`.
    ///
    /// Forma reconhecida:
    /// ```ignore
    /// <label>: CONST <alias>
    /// <label>: SPACE <alias>
    /// ```
    ///
    /// Este detector é intencionalmente sintático: ele não consulta a tabela
    /// `EQU`. A validação de existência do alias acontece no executor, para que
    /// usos indefinidos possam gerar diagnóstico próprio.
    pub(in crate::assembler::preprocessor) fn looks_like_equ_use(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [
                Token { kind: TokenKind::Ident(_), .. },
                Token { kind: TokenKind::Colon, .. },
                Token { kind: TokenKind::Ident(directive), .. },
                Token { kind: TokenKind::Ident(_), .. },
                ..
            ] if (*directive == self.language_symbols.data_directives.const_
                || *directive == self.language_symbols.data_directives.space)
        )
    }
}
