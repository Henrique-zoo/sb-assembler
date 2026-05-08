use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, ExpectedToken, InvalidParamKind,
        MacroHeaderErrorKind, PreprocessorError, PreprocessorErrorKind,
    },
    interner::Symbol,
    lexer::{Span, Token, TokenKind},
    preprocessor::{
        Preprocessor,
        ir::{MacroBodyItem, MacroBodyLine, MacroHeader, Param},
    },
};

impl Preprocessor<'_> {
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
    /// - `Ok(MacroHeader { node_id, name, params })` quando o cabeçalho é
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
        &mut self,
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
            self.language_symbols.preprocessor.macro_,
            colon_span,
        )?;
        let params = self.parse_macro_params(tail)?;
        let span = self.consumed_whole_line_span(line, label_span);
        let node_id = self.alloc_node_id(span);

        Ok(MacroHeader {
            node_id,
            name,
            params,
        })
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
            self.language_symbols.preprocessor.endmacro,
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
            Self::macro_header_sintatic_error(MacroHeaderErrorKind::InvalidLabel, Span::default())
        })?;

        match &label_token.kind {
            TokenKind::Ident(name) => Ok((*name, rest, label_token.span)),
            _ => Err(Self::macro_header_sintatic_error(
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
                            return Err(Self::invalid_param_sintatic_error(
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
                            return Err(Self::invalid_param_sintatic_error(
                                InvalidParamKind::InvalidParamIdent,
                                token.span,
                            ));
                        }
                    }
                    ParseState::ExpectCommaOrEnd => {
                        if matches!(&token.kind, TokenKind::Comma) {
                            ParseState::ExpectAmpersand
                        } else {
                            return Err(Self::directive_sintatic_error(
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
            ParseState::ExpectAmpersand if !line.is_empty() => {
                Err(Self::invalid_param_sintatic_error(
                    InvalidParamKind::UnexpectedComma,
                    last_span.unwrap_or_default(),
                ))
            }
            ParseState::ExpectParamIdent => Err(Self::invalid_param_sintatic_error(
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
    fn macro_header_sintatic_error(kind: MacroHeaderErrorKind, span: Span) -> PreprocessorError {
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
    fn invalid_param_sintatic_error(kind: InvalidParamKind, span: Span) -> PreprocessorError {
        Self::macro_header_sintatic_error(MacroHeaderErrorKind::InvalidParam(kind), span)
    }

    /// Parseia uma linha do corpo de macro em representação estruturada.
    ///
    /// Forma canônica (exemplos):
    /// ```ignore
    /// ADD &A
    /// COPY &SRC, &DST
    /// LOAD N1
    /// ```
    ///
    /// Estratégia:
    /// 1. percorre a linha com um cursor (`tail`);
    /// 2. consome um item por vez via [`Self::parse_macro_body_item`];
    /// 3. acumula os itens parseados em ordem;
    /// 4. consolida o `span` da linha inteira.
    ///
    /// Parâmetros:
    /// - `line`: conteúdo da linha (sem `NewLine`);
    /// - `terminator`: terminador original da linha no fonte.
    ///
    /// Retorno:
    /// - `Ok(MacroBodyLine { node_id, terminator, items })` quando todos os
    ///   itens da linha são válidos.
    ///
    /// Erros:
    /// - propaga a primeira falha retornada por
    ///   [`Self::parse_macro_body_item`].
    ///
    /// Efeito colateral:
    /// - nenhum. Apenas valida/converte tokens da linha.
    pub(in crate::preprocessor) fn parse_macro_body_line(
        &mut self,
        line: &[Token],
        terminator: Token,
    ) -> Result<MacroBodyLine, PreprocessorError> {
        let mut tail = line;
        let mut parsed_items = Vec::new();
        let mut fallback_span = Span::default();

        while !tail.is_empty() {
            let (item, next_tail, item_span) = self.parse_macro_body_item(tail)?;
            if parsed_items.is_empty() {
                fallback_span = item_span;
            }
            parsed_items.push(item);
            tail = next_tail;
        }

        let span = self.consumed_whole_line_span(line, fallback_span);
        let node_id = self.alloc_node_id(span);

        Ok(MacroBodyLine {
            node_id,
            terminator,
            items: parsed_items,
        })
    }

    /// Consome e parseia o próximo item do corpo de macro.
    ///
    /// Formas aceitas para o item atual:
    /// ```ignore
    /// <Ident> | <Number> | "," | ":" | "+" | "-"
    /// "&" <Ident>
    /// ```
    ///
    /// Regras:
    /// - tokens literais são preservados como
    ///   `MacroBodyItem::Literal(Token)`;
    /// - `&` inicia referência de parâmetro e delega para
    ///   [`Self::parse_macro_param_ref`].
    ///
    /// Pré-condição:
    /// - `line` não deve ser vazia.
    ///
    /// Retorno:
    /// - `Ok((item, tail, item_span))`, onde:
    ///   - `item` é o item parseado;
    ///   - `tail` é o sufixo remanescente após o consumo;
    ///   - `item_span` é o span do item consumido.
    ///
    /// Erros:
    /// - propaga erros de [`Self::parse_macro_param_ref`] quando o prefixo é
    ///   `&`.
    fn parse_macro_body_item<'a>(
        &mut self,
        line: &'a [Token],
    ) -> Result<(MacroBodyItem, &'a [Token], Span), PreprocessorError> {
        let (first, tail) = line.split_first().unwrap();

        match first.kind {
            TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Ident(_)
            | TokenKind::Number(_) => Ok((MacroBodyItem::Literal(*first), tail, first.span)),
            TokenKind::Ampersand => self.parse_macro_param_ref(tail, first.span),
            _ => unreachable!(),
        }
    }

    /// Parseia uma referência de parâmetro formal logo após `&`.
    ///
    /// Forma esperada:
    /// ```ignore
    /// &<Ident>
    /// ```
    ///
    /// Parâmetros:
    /// - `tail`: sufixo imediatamente após o token `&`;
    /// - `fallback_span`: span do `&`, usado quando não existe token seguinte.
    ///
    /// Retorno:
    /// - `Ok((MacroBodyItem::ParamRef(sym, span), tail, span))` quando o token
    ///   seguinte é `Ident`.
    ///
    /// Erros:
    /// - `InvalidDirectiveSyntax(MissingToken { directive: MacroBody, .. })`
    ///   quando não há token após `&`;
    /// - `InvalidDirectiveSyntax(UnexpectedToken { directive: MacroBody, .. })`
    ///   quando o token após `&` não é `Ident`.
    fn parse_macro_param_ref<'a>(
        &mut self,
        tail: &'a [Token],
        fallback_span: Span,
    ) -> Result<(MacroBodyItem, &'a [Token], Span), PreprocessorError> {
        let (param_token, tail) = tail.split_first().ok_or_else(|| {
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive: DirectiveKind::MacroBody,
                    expected: ExpectedToken::Ident,
                },
                fallback_span,
            )
        })?;

        if let TokenKind::Ident(sym) = param_token.kind {
            let param_ref_node_id = self.alloc_node_id(param_token.span);
            Ok((
                MacroBodyItem::ParamRef(sym, param_ref_node_id),
                tail,
                param_token.span,
            ))
        } else {
            Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::MacroBody,
                    expected: ExpectedToken::Ident,
                },
                param_token.span,
            ))
        }
    }
}
