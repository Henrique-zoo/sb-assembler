use crate::{
    assembler::errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, ExpectedToken, InvalidArgKind,
        MacroCallSyntaticErrorKind, PreprocessorError, PreprocessorErrorKind,
    },
    assembler::interner::Symbol,
    assembler::lexer::{Span, Token, TokenKind},
    assembler::preprocessor::{
        Preprocessor,
        ir::{MacroCall, MacroCallArg, Sign},
    },
};

impl Preprocessor<'_> {
    /// Parseia uma chamada de macro em formato posicional.
    ///
    /// Forma canônica esperada:
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// <MacroName> <Arg1>, +<Arg2>, -<Arg3>, ...
    /// ```
    ///
    /// Fluxo interno:
    /// 1. extrai o nome com [`Self::parse_macro_name`];
    /// 2. parseia argumentos posicionais com [`Self::parse_macro_call_args`];
    /// 3. consolida o `span` da linha inteira com
    ///    [`Self::consumed_whole_line_span`].
    ///
    /// Retorno:
    /// - `Ok(MacroCall { node_id, name, args })` quando a linha é uma chamada
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
    pub(in crate::assembler::preprocessor) fn parse_macro_call(
        &mut self,
        line: &[Token],
    ) -> Result<MacroCall, PreprocessorError> {
        let (macro_name, tail, fallback_span) = self.parse_macro_name(line)?;
        let macro_args = self.parse_macro_call_args(tail)?;
        let span = self.consumed_whole_line_span(line, fallback_span);
        let node_id = self.alloc_node_id(span);

        Ok(MacroCall {
            node_id,
            name: macro_name,
            args: macro_args,
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
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive: DirectiveKind::MacroCall,
                    expected: ExpectedToken::Ident,
                },
                Span::default(),
            )
        })?;

        match macro_name_token.kind {
            TokenKind::Ident(name) => Ok((name, tail, macro_name_token.span)),
            _ => Err(Self::directive_sintatic_error(
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
    /// - cada argumento deve ser `Ident`, `Number`, `+Number` ou `-Number`;
    /// - entre argumentos deve existir `,`;
    /// - não é permitida vírgula final sem argumento seguinte.
    ///
    /// Parâmetros:
    /// - `line`: sufixo após o nome da macro.
    ///
    /// Retorno:
    /// - `Ok(Vec<MacroCallArg>)` com os argumentos na ordem declarada;
    /// - `Ok(vec![])` quando não há argumentos.
    ///
    /// Erros:
    /// - `InvalidMacroCallSyntatic(InvalidArg(InvalidArgIdent))` quando um argumento
    ///   não é `Ident`, `Number`, `+Number` ou `-Number`;
    /// - `InvalidDirectiveSyntax(UnexpectedToken { directive: MacroCall, .. })`
    ///   quando o separador entre argumentos não é vírgula;
    /// - `InvalidMacroCallSyntatic(InvalidArg(UnexpectedComma))` para vírgula final.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_macro_call_args(
        &mut self,
        line: &[Token],
    ) -> Result<Vec<MacroCallArg>, PreprocessorError> {
        if line.is_empty() {
            return Ok(Vec::new());
        }

        let mut args = Vec::new();
        let mut tail = line;

        loop {
            let (arg, rest) = self.parse_macro_call_arg(tail)?;
            args.push(arg);

            if rest.is_empty() {
                return Ok(args);
            }

            let (comma, rest) = rest.split_first().unwrap_or_else(|| unreachable!());
            if !matches!(comma.kind, TokenKind::Comma) {
                return Err(Self::directive_sintatic_error(
                    DirectiveSyntaxErrorKind::UnexpectedToken {
                        directive: DirectiveKind::MacroCall,
                        expected: ExpectedToken::Exact(TokenKind::Comma),
                    },
                    comma.span,
                ));
            }

            if rest.is_empty() {
                return Err(Self::invalid_arg_sintatic_error(
                    InvalidArgKind::UnexpectedComma,
                    comma.span,
                ));
            }

            tail = rest;
        }
    }

    /// Parseia um único argumento posicional no início de `line`.
    ///
    /// Formas aceitas:
    /// - `Ident`;
    /// - `Number`;
    /// - `+Number`;
    /// - `-Number`.
    ///
    /// Retorno:
    /// - `Ok((MacroCallArg, tail))` quando o prefixo representa argumento
    ///   válido.
    ///
    /// Erros:
    /// - `InvalidMacroCallSyntatic(InvalidArg(InvalidArgIdent))` quando o
    ///   prefixo não representa argumento posicional aceito.
    ///
    /// Efeito colateral:
    /// - nenhum.
    fn parse_macro_call_arg<'a>(
        &mut self,
        line: &'a [Token],
    ) -> Result<(MacroCallArg, &'a [Token]), PreprocessorError> {
        let (first, tail) = line.split_first().ok_or_else(|| {
            Self::invalid_arg_sintatic_error(InvalidArgKind::InvalidArgIdent, Span::default())
        })?;

        match first.kind {
            TokenKind::Ident(sym) => Ok((
                MacroCallArg::Ident {
                    sym,
                    node_id: self.alloc_node_id(first.span),
                },
                tail,
            )),
            TokenKind::Number(sym) => Ok((
                MacroCallArg::UnsignedNumber {
                    sym,
                    node_id: self.alloc_node_id(first.span),
                },
                tail,
            )),
            TokenKind::Plus | TokenKind::Minus => {
                let (number_token, rest) = tail.split_first().ok_or_else(|| {
                    Self::invalid_arg_sintatic_error(InvalidArgKind::InvalidArgIdent, first.span)
                })?;

                let TokenKind::Number(sym) = number_token.kind else {
                    return Err(Self::invalid_arg_sintatic_error(
                        InvalidArgKind::InvalidArgIdent,
                        number_token.span,
                    ));
                };

                let sign = match first.kind {
                    TokenKind::Plus => Sign::Plus,
                    TokenKind::Minus => Sign::Minus,
                    _ => unreachable!(),
                };

                let span = Self::span_from_bounds(first.span, number_token.span);
                Ok((
                    MacroCallArg::SignedNumber {
                        sign,
                        sym,
                        node_id: self.alloc_node_id(span),
                    },
                    rest,
                ))
            }
            _ => Err(Self::invalid_arg_sintatic_error(
                InvalidArgKind::InvalidArgIdent,
                first.span,
            )),
        }
    }

    /// Constrói erro específico da família de chamadas de macro.
    ///
    /// Encapsula `MacroCallSyntaticErrorKind` em
    /// `PreprocessorErrorKind::InvalidMacroCallSyntatic`, preservando o `span`.
    fn macro_call_sintatic_error(
        kind: MacroCallSyntaticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidMacroCallSyntatic(kind),
            span,
        }
    }

    /// Constrói erro de argumento inválido para chamada de macro.
    ///
    /// É um atalho para `InvalidMacroCallSyntatic(InvalidArg(...))`, usado no parser
    /// de argumentos para manter o fluxo declarativo.
    ///
    /// Contrato:
    /// - não altera estado interno;
    /// - preserva o `span` recebido.
    #[inline]
    fn invalid_arg_sintatic_error(kind: InvalidArgKind, span: Span) -> PreprocessorError {
        Self::macro_call_sintatic_error(MacroCallSyntaticErrorKind::InvalidArg(kind), span)
    }
}
