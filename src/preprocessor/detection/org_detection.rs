use crate::{
    lexer::{Token, TokenKind},
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Indica se a linha parece uma diretiva `ORG`.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// ORG <valor>
    /// ```
    ///
    /// Critério de triagem:
    /// - o prefixo deve ser `ORG`;
    /// - aceita qualquer sufixo após o prefixo reconhecido.
    ///
    /// Contrato:
    /// - retorna `true` para linhas candidatas, inclusive malformadas;
    /// - não faz validação sintática estrita.
    pub(in crate::preprocessor) fn looks_like_org_line(&self, line: &[Token]) -> bool {
        matches!(
            line,
            [Token { kind: TokenKind::Ident(sym), .. }, ..]
            if *sym == self.keywords.org_kw
        )
    }
}
