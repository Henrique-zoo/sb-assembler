use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, InvalidParamKind, MacroHeaderErrorKind,
        PreprocessorError, PreprocessorErrorKind,
    },
    interner::Symbol,
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        Preprocessor,
        ir::{MacroHeader, Param},
    },
};

impl Preprocessor {
    /// Faz o parsing completo do cabeçalho de macro.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// <Label>: MACRO
    /// <Label>: MACRO &P1, &P2, ...
    /// ```
    ///
    /// Fluxo interno:
    /// 1. valida e extrai label com `parse_macro_label`;
    /// 2. exige `:` com `consume_required_token`;
    /// 3. exige keyword `MACRO` com `consume_keyword`;
    /// 4. parseia parâmetros com `parse_macro_params`.
    ///
    /// Estilo de implementação:
    /// - composição funcional de etapas pequenas, em pipeline;
    /// - cada etapa consome/valida parte da entrada e devolve o restante;
    /// - propagação de falha imediata via `?` (estilo parser combinator manual).
    ///
    /// Retorno:
    /// - `Ok(MacroHeader { name, params, span })` quando o cabeçalho é
    ///   sintaticamente válido.
    ///
    /// Erros:
    /// - propaga erros de qualquer etapa do pipeline (`InvalidLabel`,
    ///   `MissingToken`/`UnexpectedToken` em tokens obrigatórios e
    ///   erros de parâmetro).
    ///
    /// Efeito colateral:
    /// - nenhum. A função apenas valida e extrai estrutura.
    pub(in crate::preprocessor) fn parse_macro_header(
        &self,
        line: &[Token],
    ) -> Result<MacroHeader, PreprocessorError> {
        let (name, tail, label_span) = self.parse_macro_label(line)?;
        let (tail, colon_span) = self.consume_required_token(
            tail,
            DirectiveKind::MacroHeader,
            TokenKind::Colon,
            label_span,
        )?;
        let (tail, _) = self.consume_keyword(
            tail,
            DirectiveKind::MacroHeader,
            self.keywords.macro_kw,
            colon_span,
        )?;
        let params = self.parse_macro_params(tail)?;
        let span = self.consumed_whole_line_span(line, label_span);

        Ok(MacroHeader { name, params, span })
    }

    /// Faz o parsing completo da linha de encerramento de macro.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// ENDMACRO
    /// ```
    ///
    /// Fluxo interno:
    /// 1. exige keyword `ENDMACRO` com `consume_keyword`;
    /// 2. garante ausência de sufixo com `ensure_no_trailing_tokens`.
    ///
    /// Retorno:
    /// - `Ok(())` quando a linha é exatamente `ENDMACRO`.
    ///
    /// Erros:
    /// - propaga falhas de `consume_keyword` (token ausente/inesperado no
    ///   prefixo);
    /// - propaga `TrailingTokens` quando há tokens após `ENDMACRO`.
    ///
    /// Efeito colateral:
    /// - nenhum. A função apenas valida sintaxe.
    pub(in crate::preprocessor) fn parse_endmacro_line(
        &self,
        line: &[Token],
    ) -> Result<(), PreprocessorError> {
        let (tail, _) = self.consume_keyword(
            line,
            DirectiveKind::EndMacro,
            self.keywords.endmacro_kw,
            Span::default(),
        )?;

        self.ensure_no_trailing_tokens(tail, DirectiveKind::EndMacro)?;

        Ok(())
    }

    /// Lê e valida o rótulo inicial do cabeçalho de macro.
    ///
    /// Forma esperada (prefixo):
    /// ```ignore
    /// <Label> ...
    /// ```
    ///
    /// Retorno:
    /// - `Ok((name, rest, label_span))`, onde:
    ///   - `name` é o símbolo internado da label;
    ///   - `rest` é o sufixo da linha após a label;
    ///   - `label_span` é o `Span` da própria label, útil como fallback.
    ///
    /// Erros:
    /// - `InvalidLabel` quando a linha está vazia;
    /// - `InvalidLabel` quando o primeiro token não é `Ident`.
    ///
    /// Efeito colateral:
    /// - nenhum. A função é puramente de leitura/validação.
    fn parse_macro_label<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(Symbol, &'a [Token], Span), PreprocessorError> {
        // Esse erro (não haver label) deveria ser unreachable, já que o `looks_like_macro_header(..)` só
        // reconhece como "parecido" com macro header se o primeiro token for `Ident(_)`
        // e o preprocessador só chama o `parse_macro_header` se o `looks_like_*` reconhecer
        // Por isso, podemos utilizar o `Span::default()` aqui
        let (label_token, rest) = line.split_first().ok_or_else(|| {
            Self::macro_header_error(MacroHeaderErrorKind::InvalidLabel, Span::default())
        })?;

        match &label_token.kind {
            TokenKind::Ident(name) => Ok((*name, rest, label_token.span)),
            _ => Err(Self::macro_header_error(
                MacroHeaderErrorKind::InvalidLabel,
                label_token.span,
            )),
        }
    }

    /// Parseia a lista de parâmetros posicionais após a keyword `MACRO`.
    ///
    /// Forma canônica na linguagem:
    /// ```ignore
    /// MACRO
    /// MACRO &A
    /// MACRO &A, &B, &C
    /// ```
    ///
    /// A estratégia usa uma pequena máquina de estados sobre o iterador:
    /// - `ExpectAmpersand`: espera `&` iniciando o próximo parâmetro;
    /// - `ExpectParamIdent`: espera `Ident` do parâmetro após `&`;
    /// - `ExpectCommaOrEnd`: espera `,` para continuar ou fim da lista.
    ///
    /// Esse desenho permite detectar o primeiro erro com precisão, retornando
    /// cedo sem varrer tokens desnecessários.
    ///
    /// Retorno:
    /// - `Ok(Vec<Param>)` com os símbolos internados dos parâmetros, na ordem
    ///   declarada.
    ///
    /// Erros:
    /// - `InvalidParam(NoAmpersand)` quando um parâmetro não começa com `&`;
    /// - `InvalidParam(UnexpectedComma)` quando há vírgula final sem próximo parâmetro;
    /// - `InvalidParam(InvalidParamIdent)` quando o token após `&` não é `Ident`;
    /// - `InvalidDirectiveSyntax(TrailingTokens { directive: MacroHeader })`
    ///   quando aparece token inválido onde só `,` ou fim seriam aceitos.
    ///
    /// Efeito colateral:
    /// - nenhum. A função não altera estado global do preprocessor.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// // Sem parâmetros
    /// assert_eq!(preprocessor.parse_macro_params(&[])?, vec![]);
    ///
    /// // Um parâmetro
    /// assert_eq!(
    ///     preprocessor.parse_macro_params(&[
    ///         Token::new(TokenKind::Ampersand, span),
    ///         Token::new(TokenKind::Ident(a), span),
    ///     ])?,
    ///     vec![a]
    /// );
    ///
    /// // Dois parâmetros
    /// assert_eq!(
    ///     preprocessor.parse_macro_params(&[
    ///         Token::new(TokenKind::Ampersand, span),
    ///         Token::new(TokenKind::Ident(a), span),
    ///         Token::new(TokenKind::Comma, span),
    ///         Token::new(TokenKind::Ampersand, span),
    ///         Token::new(TokenKind::Ident(b), span),
    ///     ])?,
    ///     vec![a, b]
    /// );
    /// ```
    fn parse_macro_params(&self, line: &[Token]) -> Result<Vec<Param>, PreprocessorError> {
        #[derive(Clone, Copy)]
        enum ParseState {
            ExpectAmpersand,
            ExpectParamIdent,
            ExpectCommaOrEnd,
        }

        let (params, state, last_span) = line.iter().try_fold(
            (Vec::new(), ParseState::ExpectAmpersand, None),
            |(mut params, state, _), token| {
                let next_state = match state {
                    ParseState::ExpectAmpersand => {
                        if matches!(&token.kind, TokenKind::Ampersand) {
                            ParseState::ExpectParamIdent
                        } else {
                            return Err(Self::invalid_param_error(
                                InvalidParamKind::NoAmpersand,
                                token.span,
                            ));
                        }
                    }
                    ParseState::ExpectParamIdent => {
                        if let TokenKind::Ident(param) = &token.kind {
                            params.push(*param);
                            ParseState::ExpectCommaOrEnd
                        } else {
                            return Err(Self::invalid_param_error(
                                InvalidParamKind::InvalidParamIdent,
                                token.span,
                            ));
                        }
                    }
                    ParseState::ExpectCommaOrEnd => {
                        if matches!(&token.kind, TokenKind::Comma) {
                            ParseState::ExpectAmpersand
                        } else {
                            return Err(Self::directive_syntax_error(
                                DirectiveSyntaxErrorKind::TrailingTokens {
                                    directive: DirectiveKind::MacroHeader,
                                },
                                token.span,
                            ));
                        }
                    }
                };

                Ok((params, next_state, Some(token.span)))
            },
        )?;

        match state {
            ParseState::ExpectAmpersand if !line.is_empty() => Err(Self::invalid_param_error(
                InvalidParamKind::UnexpectedComma,
                last_span.unwrap_or_default(),
            )),
            ParseState::ExpectParamIdent => Err(Self::invalid_param_error(
                InvalidParamKind::InvalidParamIdent,
                last_span.unwrap_or_default(),
            )),
            ParseState::ExpectCommaOrEnd => Ok(params),
            ParseState::ExpectAmpersand => Ok(params),
        }
    }

    /// Constrói um erro de cabeçalho de macro já envelopado em `PreprocessorError`.
    ///
    /// Este helper centraliza a criação de erros de parsing de cabeçalho e
    /// evita repetição de boilerplate.
    ///
    /// Contrato:
    /// - não altera estado do pré-processador;
    /// - apenas encapsula `MacroHeaderErrorKind` em
    ///   `PreprocessorErrorKind::InvalidMacroHeader`.
    fn macro_header_error(kind: MacroHeaderErrorKind, span: Span) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidMacroHeader(kind),
            span,
        }
    }

    /// Constrói erro de parâmetro inválido no contexto de cabeçalho de macro.
    ///
    /// É um atalho para `InvalidMacroHeader(InvalidParam(...))`, usado no
    /// parser de parâmetros para manter o código mais declarativo.
    ///
    /// Contrato:
    /// - não altera estado interno;
    /// - preserva o `span` recebido para diagnóstico.
    #[inline]
    fn invalid_param_error(kind: InvalidParamKind, span: Span) -> PreprocessorError {
        Self::macro_header_error(MacroHeaderErrorKind::InvalidParam(kind), span)
    }
}
