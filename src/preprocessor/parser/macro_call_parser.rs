use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, InvalidArgKind, MacroCallErrorKind,
        PreprocessorError, PreprocessorErrorKind,
    },
    interner::Symbol,
    lexer::{Span, Token, TokenKind},
    preprocessor::{Preprocessor, ir::MacroCall},
};

impl Preprocessor {
    /// Parseia uma chamada de macro em formato posicional.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// ```
    ///
    /// Fluxo interno:
    /// 1. extrai o nome com [`Self::parse_macro_name`];
    /// 2. parseia argumentos posicionais com [`Self::parse_macro_args`];
    /// 3. consolida o `span` da linha inteira com
    ///    [`Self::consumed_whole_line_span`].
    ///
    /// Retorno:
    /// - `Ok(MacroCall { name, args, span })` quando a linha é uma chamada
    ///   sintaticamente válida.
    ///
    /// Erros:
    /// - propaga erros de nome ausente/inválido;
    /// - propaga erros de lista de argumentos (tipo inválido, vírgula extra ou
    ///   token inesperado entre argumentos).
    ///
    /// Efeito colateral:
    /// - nenhum. Apenas valida e monta IR.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let call = preprocessor.parse_macro_call(tokens)?;
    /// // Exemplo de forma válida:
    /// // ROT R1, R2
    /// ```
    pub(in crate::preprocessor) fn parse_macro_call(
        &self,
        line: &[Token],
    ) -> Result<MacroCall, PreprocessorError> {
        let (macro_name, tail, fallback_span) = self.parse_macro_name(line)?;
        let macro_args = self.parse_macro_args(tail)?;
        let span = self.consumed_whole_line_span(line, fallback_span);

        Ok(MacroCall {
            name: macro_name,
            args: macro_args,
            span,
        })
    }

    /// Extrai e valida o identificador da macro chamada.
    ///
    /// Forma esperada (prefixo):
    /// ```ignore
    /// <MacroName> ...
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: linha original da chamada ainda não consumida.
    ///
    /// Retorno:
    /// - `Ok((macro_name, tail, macro_name_span))`, onde:
    ///   - `macro_name` é o símbolo internado do nome da macro;
    ///   - `tail` é o sufixo após o nome;
    ///   - `macro_name_span` é o `Span` do nome extraído.
    ///
    /// Erros:
    /// - `MissingToken` quando a linha está vazia;
    /// - `InvalidDeclaration { directive: MacroCall }` quando o primeiro token
    ///   não é `Ident`.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_macro_name<'a>(
        &self,
        line: &'a [Token],
    ) -> Result<(Symbol, &'a [Token], Span), PreprocessorError> {
        let (macro_name_token, tail) = line.split_first().ok_or_else(|| {
            Self::directive_syntax_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive: DirectiveKind::MacroCall,
                    token_missed: None,
                },
                Span::default(),
            )
        })?;

        match macro_name_token.kind {
            TokenKind::Ident(name) => Ok((name, tail, macro_name_token.span)),
            _ => Err(Self::directive_syntax_error(
                DirectiveSyntaxErrorKind::InvalidDeclaration {
                    directive: DirectiveKind::MacroCall,
                },
                macro_name_token.span,
            )),
        }
    }

    /// Parseia os argumentos posicionais da chamada.
    ///
    /// Formas aceitas:
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// ```
    ///
    /// Regras sintáticas:
    /// - cada argumento deve ser `Ident` ou `Number`;
    /// - entre argumentos deve existir `,`;
    /// - não é permitida vírgula final sem argumento seguinte.
    ///
    /// Estratégia:
    /// - usa `try_fold` com uma máquina de estados pequena:
    ///   - `ExpectArgIdent`: espera argumento;
    ///   - `ExpectCommaOrEnd`: espera `,` ou fim da lista.
    ///
    /// Parâmetros:
    /// - `line`: sufixo após o nome da macro.
    ///
    /// Retorno:
    /// - `Ok(Vec<Symbol>)` com os argumentos na ordem declarada;
    /// - `Ok(vec![])` quando não há argumentos.
    ///
    /// Erros:
    /// - `InvalidMacroCall(InvalidArg(InvalidArgIdent))` quando um argumento
    ///   não é `Ident` nem `Number`;
    /// - `InvalidDirectiveSyntax(UnexpectedToken { directive: MacroCall, .. })`
    ///   quando o separador entre argumentos não é vírgula;
    /// - `InvalidMacroCall(InvalidArg(UnexpectedComma))` para vírgula final.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_macro_args(&self, line: &[Token]) -> Result<Vec<Symbol>, PreprocessorError> {
        if line.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(Clone, Copy)]
        enum ParseState {
            ExpectArgIdent,
            ExpectCommaOrEnd,
        }

        let (args, state, last_span) = line.iter().try_fold(
            (Vec::new(), ParseState::ExpectArgIdent, Span::default()),
            |(mut args, state, _), token| {
                let next_state = match state {
                    ParseState::ExpectArgIdent => {
                        args.push(self.parse_macro_call_arg(token)?);
                        ParseState::ExpectCommaOrEnd
                    }
                    ParseState::ExpectCommaOrEnd => {
                        if matches!(&token.kind, TokenKind::Comma) {
                            ParseState::ExpectArgIdent
                        } else {
                            return Err(Self::directive_syntax_error(
                                DirectiveSyntaxErrorKind::UnexpectedToken {
                                    directive: DirectiveKind::MacroCall,
                                    expected_token: self.fixed_symbols.comma,
                                },
                                token.span,
                            ));
                        }
                    }
                };

                Ok((args, next_state, token.span))
            },
        )?;

        match state {
            ParseState::ExpectArgIdent => Err(Self::invalid_arg_error(
                InvalidArgKind::UnexpectedComma,
                last_span,
            )),
            ParseState::ExpectCommaOrEnd => Ok(args),
        }
    }

    /// Parseia um único argumento posicional.
    ///
    /// Formas aceitas:
    /// - `Ident`;
    /// - `Number`.
    ///
    /// Retorno:
    /// - `Ok(Symbol)` quando o token representa um argumento válido.
    ///
    /// Erros:
    /// - `InvalidMacroCall(InvalidArg(InvalidArgIdent))` quando o token não é
    ///   argumento posicional aceito.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_macro_call_arg(&self, token: &Token) -> Result<Symbol, PreprocessorError> {
        match token.kind {
            TokenKind::Ident(sym) | TokenKind::Number(sym) => Ok(sym),
            _ => Err(Self::invalid_arg_error(
                InvalidArgKind::InvalidArgIdent,
                token.span,
            )),
        }
    }

    /// Constrói erro específico da família de chamadas de macro.
    ///
    /// Encapsula `MacroCallErrorKind` em
    /// `PreprocessorErrorKind::InvalidMacroCall`, preservando o `span`.
    fn macro_call_error(kind: MacroCallErrorKind, span: Span) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidMacroCall(kind),
            span,
        }
    }

    /// Constrói erro de argumento inválido para chamada de macro.
    ///
    /// É um atalho para `InvalidMacroCall(InvalidArg(...))`, usado no parser
    /// de argumentos para manter o fluxo declarativo.
    ///
    /// Contrato:
    /// - não altera estado interno;
    /// - preserva o `span` recebido.
    #[inline]
    fn invalid_arg_error(kind: InvalidArgKind, span: Span) -> PreprocessorError {
        Self::macro_call_error(MacroCallErrorKind::InvalidArg(kind), span)
    }
}
