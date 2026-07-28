//! Parser sintático do pré-processador.
//!
//! Este submódulo adota uma abordagem **function-driven**:
//! - cada função resolve uma etapa pequena e bem definida do parsing;
//! - as etapas são compostas em pipeline;
//! - erros são propagados de forma declarativa com `Result` + `?`.
//!
//! Embora não use uma biblioteca formal de parser combinators, o desenho segue
//! esse estilo: funções pequenas "combinam" entre si para construir um parser
//! maior.
//!
//! Exemplo de composição em `parse_macro_header`:
//! 1. `parse_macro_label` extrai a label e o restante da entrada;
//! 2. `consume_required_token` valida e consome `:`;
//! 3. `consume_keyword` valida e consome `MACRO`;
//! 4. `parse_macro_params` interpreta a lista de parâmetros.
//!
//! Essa organização melhora legibilidade, testabilidade e manutenção:
//! - o comportamento de cada etapa fica isolado;
//! - o fluxo principal vira uma composição linear de transformações;
//! - o primeiro erro relevante interrompe o pipeline com contexto adequado.
//!
//! Organização por assunto:
//! - `macro_parser`: cabeçalho de macro (`<Label>: MACRO ...`) e `ENDMACRO`;
//! - `macro_call_parser`: sintaxe das chamadas de macro;
//! - `equ_parser`: diretiva `EQU`;
//! - `if_parser`: diretiva `IF`;
//! - `section_parser`: diretivas de seção (`SECTION TEXT`/`SECTION DATA`), que
//!   validam troca de contexto e não geram linha de saída.
//!
//! Integração com IR:
//! - os parsers retornam estruturas em [`crate::assembler::preprocessor::ir`], separando
//!   validação sintática de execução semântica.

use crate::{
    assembler::errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, ExpectedToken, PreprocessorError,
        PreprocessorErrorKind,
    },
    assembler::interner::Symbol,
    assembler::language::numeric_literals::{NumberSign, NumericLiteral},
    assembler::lexer::{Span, Token, TokenKind},
    assembler::preprocessor::{Preprocessor, ir::Operand},
};

mod equ_parser;
mod if_parser;
mod macro_call_parser;
mod macro_parser;
mod section_parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperandParseErrorKind {
    Missing,
    InvalidType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OperandParseError {
    kind: OperandParseErrorKind,
    span: Span,
}

impl Preprocessor<'_> {
    /// Constrói um erro sintático genérico de diretiva com `span`.
    ///
    /// Parâmetros:
    /// - `kind`: categoria sintática da falha (`MissingToken`, `TrailingTokens`,
    ///   `InvalidDeclaration` etc.);
    /// - `span`: posição da entrada associada ao diagnóstico.
    ///
    /// Retorno:
    /// - `PreprocessorError` já envelopado em
    ///   `PreprocessorErrorKind::InvalidDirectiveSyntax`.
    ///
    /// Efeito colateral:
    /// - nenhum. Este helper apenas encapsula dados.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let err = Preprocessor::directive_sintatic_error(
    ///     DirectiveSyntaxErrorKind::TrailingTokens {
    ///         directive: DirectiveKind::If,
    ///     },
    ///     token.span,
    /// );
    /// ```
    fn directive_sintatic_error(kind: DirectiveSyntaxErrorKind, span: Span) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidDirectiveSyntax(kind),
            span,
        }
    }

    /// Consome uma keyword (`TokenKind::Ident`) esperada no início da fatia.
    ///
    /// Forma esperada (prefixo):
    /// ```ignore
    /// <KeywordEsperada> ...
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: fatia de tokens a ser consumida;
    /// - `directive`: diretiva associada ao consumo (usada para construir erro);
    /// - `expected_keyword`: símbolo internado da keyword esperada;
    /// - `fallback_span`: span usado quando `line` está vazia.
    ///
    /// Retorno:
    /// - `Ok((tail, kw_span))`, onde:
    ///   - `tail` é o sufixo da linha após consumir a keyword;
    ///   - `kw_span` é o span da keyword consumida.
    ///
    /// Erros:
    /// - `InvalidDirectiveSyntax(MissingToken { directive, expected })`
    ///   quando `line` está vazia;
    /// - `InvalidDirectiveSyntax(UnexpectedToken { directive, expected })`
    ///   quando o primeiro token não é `Ident(expected_keyword)`.
    ///
    /// Efeito colateral:
    /// - nenhum. Não altera estado global do pré-processador.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// // Uso típico no parser de IF:
    /// let (tail, if_span) = self.consume_keyword(
    ///     line,
    ///     DirectiveKind::If,
    ///     self.language_symbols.preprocessor.if_,
    ///     Span::default(),
    /// )?;
    /// ```
    fn consume_keyword<'a>(
        &self,
        line: &'a [Token],
        directive: DirectiveKind,
        expected_keyword: Symbol,
        fallback_span: Span,
    ) -> Result<(&'a [Token], Span), PreprocessorError> {
        let (token, tail) = line.split_first().ok_or_else(|| {
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive,
                    expected: ExpectedToken::Keyword(expected_keyword),
                },
                fallback_span,
            )
        })?;

        if matches!(&token.kind, TokenKind::Ident(sym) if *sym == expected_keyword) {
            Ok((tail, token.span))
        } else {
            Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive,
                    expected: ExpectedToken::Keyword(expected_keyword),
                },
                token.span,
            ))
        }
    }

    /// Consome um token obrigatório específico no início da fatia.
    ///
    /// Forma esperada (prefixo):
    /// ```ignore
    /// <TokenEsperado> ...
    /// ```
    ///
    /// Uso recomendado:
    /// - pontuação e tokens fixos de gramática (`:`, `,`, `&`, `+`, `-` etc.).
    ///
    /// Parâmetros:
    /// - `line`: fatia de tokens a ser consumida;
    /// - `directive`: diretiva associada ao consumo (usada para construir erro);
    /// - `expected_token`: token obrigatório esperado no início de `line`;
    /// - `fallback_span`: span usado quando `line` está vazia.
    ///
    /// Retorno:
    /// - `Ok((tail, token_span))`, onde:
    ///   - `tail` é o sufixo da linha após consumir o token;
    ///   - `token_span` é o span do token consumido.
    ///
    /// Erros:
    /// - `InvalidDirectiveSyntax(MissingToken { directive, expected })`
    ///   quando `line` está vazia;
    /// - `InvalidDirectiveSyntax(UnexpectedToken { directive, expected })`
    ///   quando o primeiro token difere de `expected_token`.
    ///
    /// Efeito colateral:
    /// - nenhum. Não altera estado global.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// // Uso típico no parser de cabeçalho de macro:
    /// let (tail, colon_span) = self.consume_required_token(
    ///     rest,
    ///     DirectiveKind::MacroHeader,
    ///     TokenKind::Colon,
    ///     fallback_span,
    /// )?;
    /// ```
    fn consume_required_token<'a>(
        &self,
        line: &'a [Token],
        directive: DirectiveKind,
        expected_token: TokenKind,
        fallback_span: Span,
    ) -> Result<(&'a [Token], Span), PreprocessorError> {
        let (token, tail) = line.split_first().ok_or_else(|| {
            Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::MissingToken {
                    directive,
                    expected: ExpectedToken::from_token_kind(expected_token),
                },
                fallback_span,
            )
        })?;

        if token.kind == expected_token {
            Ok((tail, token.span))
        } else {
            Err(Self::directive_sintatic_error(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive,
                    expected: ExpectedToken::from_token_kind(expected_token),
                },
                token.span,
            ))
        }
    }

    /// Garante que o token da posição atual (`line[0]`) não seja proibido.
    ///
    /// Este helper valida apenas o primeiro token da fatia (`line.first()`),
    /// sem consumir a entrada. Ele é útil para detectar erros sintáticos
    /// clássicos logo no ponto atual do parser (por exemplo, `:` após alias em
    /// `EQU`).
    ///
    /// Forma esperada:
    /// ```ignore
    /// line.first() != Some(forbidden_kind)
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: fatia de tokens cujo elemento atual será validado;
    /// - `forbidden_kind`: token proibido na posição atual;
    /// - `directive`: diretiva associada ao diagnóstico.
    ///
    /// Retorno:
    /// - `Ok(())` quando:
    ///   - `line` está vazia; ou
    ///   - o primeiro token não é `forbidden_kind`;
    /// - `Err(...)` quando o primeiro token é `forbidden_kind`.
    ///
    /// Erros:
    /// - `InvalidDirectiveSyntax(ForbiddenToken { directive })` quando a
    ///   validação falha, usando o `span` do próprio token proibido.
    ///
    /// Efeito colateral:
    /// - nenhum. A fatia não é consumida/modificada.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// // Em `EQU`, impedir "<Alias> : EQU <Valor>" logo após o alias.
    /// self.ensure_token_is_not(rest, TokenKind::Colon, DirectiveKind::Equ)?;
    /// ```
    fn ensure_token_is_not(
        &self,
        line: &[Token],
        forbidden_kind: TokenKind,
        directive: DirectiveKind,
    ) -> Result<(), PreprocessorError> {
        if let Some(token) = line.first() {
            if token.kind == forbidden_kind {
                return Err(Self::directive_sintatic_error(
                    DirectiveSyntaxErrorKind::ForbiddenToken { directive },
                    token.span,
                ));
            }
        }

        Ok(())
    }

    /// Garante ausência de tokens remanescentes após o parse de uma diretiva.
    ///
    /// Forma esperada:
    /// ```ignore
    /// <diretiva completamente consumida>
    /// ```
    ///
    /// Parâmetros:
    /// - `line`: sufixo de tokens remanescente após o parsing da diretiva;
    /// - `directive`: diretiva associada ao diagnóstico de trailing tokens.
    ///
    /// Retorno:
    /// - `Ok(())` quando `line` está vazia;
    /// - erro quando existe ao menos um token extra.
    ///
    /// Erros:
    /// - `InvalidDirectiveSyntax(TrailingTokens { directive })` apontando para
    ///   o primeiro token remanescente.
    ///
    /// Efeito colateral:
    /// - nenhum.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// self.ensure_no_trailing_tokens(tail, DirectiveKind::If)?;
    /// self.ensure_no_trailing_tokens(tail, DirectiveKind::Section)?;
    /// ```
    fn ensure_no_trailing_tokens(
        &self,
        line: &[Token],
        directive: DirectiveKind,
    ) -> Result<(), PreprocessorError> {
        if line.is_empty() {
            return Ok(());
        }

        Err(Self::directive_sintatic_error(
            DirectiveSyntaxErrorKind::TrailingTokens { directive },
            line[0].span,
        ))
    }

    /// Calcula o `Span` da faixa efetivamente consumida no parsing de uma linha.
    ///
    /// A função assume o padrão usado nos parsers deste módulo:
    /// - `line` é a fatia original recebida pela função de parsing;
    /// - `tail` é o sufixo remanescente após consumir o prefixo válido.
    ///
    /// Com isso, o span resultante cobre do primeiro token consumido ao último
    /// token consumido do prefixo.
    ///
    /// Parâmetros:
    /// - `line`: fatia original da diretiva;
    /// - `tail`: sufixo remanescente após o consumo;
    /// - `fallback_span`: span retornado quando nada foi consumido.
    ///
    /// Retorno:
    /// - `Span` da faixa consumida;
    /// - `fallback_span` quando `line` está vazia ou nenhum token foi consumido.
    ///
    /// Efeito colateral:
    /// - nenhum.
    ///
    /// # Exemplos
    /// ```rust,ignore
    /// let full_span = self.consumed_prefix_span(line, tail, keyword_span);
    /// ```
    fn consumed_prefix_span(&self, line: &[Token], tail: &[Token], fallback_span: Span) -> Span {
        if line.is_empty() {
            return fallback_span;
        }

        let consumed_len = line.len().saturating_sub(tail.len());
        if consumed_len == 0 {
            return fallback_span;
        }

        let first = line[0].span;
        let last = line[consumed_len - 1].span;
        Self::span_from_bounds(first, last)
    }

    /// Calcula o `Span` da linha inteira quando o parser consome tudo.
    ///
    /// Este helper é apropriado para parsers cujo contrato é:
    /// - ou a linha inteira é consumida com sucesso;
    /// - ou a função retorna erro.
    ///
    /// Nessas situações, não há `tail` útil para propagar e usar
    /// [`Self::consumed_prefix_span`].
    ///
    /// Parâmetros:
    /// - `line`: linha original recebida pelo parser;
    /// - `fallback_span`: span retornado quando `line` está vazia.
    ///
    /// Retorno:
    /// - `Span` cobrindo do primeiro ao último token da linha;
    /// - `fallback_span` quando a linha está vazia.
    fn consumed_whole_line_span(&self, line: &[Token], fallback_span: Span) -> Span {
        match (line.first(), line.last()) {
            (Some(first), Some(last)) => Self::span_from_bounds(first.span, last.span),
            _ => fallback_span,
        }
    }

    /// Combina dois spans em um único intervalo contínuo.
    ///
    /// O span final inicia em `start` e vai até o fim de `end`, preservando
    /// linha/coluna iniciais de `start`.
    fn span_from_bounds(start: Span, end: Span) -> Span {
        let end_pos = end.pos.saturating_add(end.len);
        Span {
            pos: start.pos,
            line: start.line,
            column: start.column,
            len: end_pos.saturating_sub(start.pos),
        }
    }

    /// Parseia um operando simples (`Ident`, `Number`, `+Number` ou `-Number`).
    ///
    /// Este helper centraliza a gramática mínima de operandos usada por
    /// diretivas como `IF` e `EQU`, retornando também o sufixo não consumido.
    ///
    /// Contrato:
    /// - entrada vazia => `Err(Missing)` com `fallback_span`;
    /// - token inicial inválido => `Err(InvalidType)` no `span` do token;
    /// - sinal sem número (`+`/`-` no fim ou seguido por token não numérico)
    ///   => `Err(InvalidType)` no `span` do problema.
    ///
    /// Retorno:
    /// - `Ok((operand, tail, consumed_span))` quando o prefixo é válido;
    /// - `Err(OperandParseError)` quando o prefixo viola a gramática.
    fn parse_operand<'a>(
        &mut self,
        line: &'a [Token],
        fallback_span: Span,
    ) -> Result<(Operand, &'a [Token], Span), OperandParseError> {
        let (first, tail) = line.split_first().ok_or(OperandParseError {
            kind: OperandParseErrorKind::Missing,
            span: fallback_span,
        })?;

        match first.kind {
            TokenKind::Ident(sym) => {
                let node_id = self.alloc_node_id(first.span);
                Ok((Operand::Ident { sym, node_id }, tail, first.span))
            }
            TokenKind::Number(_) => {
                let operand = self.parse_unsigned(first);
                Ok((operand, tail, first.span))
            }
            TokenKind::Plus | TokenKind::Minus => {
                let (num_tok, rest) = tail.split_first().ok_or(OperandParseError {
                    kind: OperandParseErrorKind::InvalidType,
                    span: first.span,
                })?;
                if !matches!(num_tok.kind, TokenKind::Number(_)) {
                    return Err(OperandParseError {
                        kind: OperandParseErrorKind::InvalidType,
                        span: num_tok.span,
                    });
                }

                let operand = self.parse_signed(first, num_tok);
                let consumed_span = Self::span_from_bounds(first.span, num_tok.span);
                Ok((operand, rest, consumed_span))
            }
            _ => Err(OperandParseError {
                kind: OperandParseErrorKind::InvalidType,
                span: first.span,
            }),
        }
    }

    fn parse_signed(&mut self, sign_tok: &Token, num_tok: &Token) -> Operand {
        let sign = match sign_tok.kind {
            TokenKind::Plus => NumberSign::Plus,
            TokenKind::Minus => NumberSign::Minus,
            _ => unreachable!(),
        };

        if let TokenKind::Number(sym) = num_tok.kind {
            let span = Self::span_from_bounds(sign_tok.span, num_tok.span);
            Operand::Number {
                number: NumericLiteral::signed(sign, sym),
                node_id: self.alloc_node_id(span),
            }
        } else {
            unreachable!()
        }
    }

    fn parse_unsigned(&mut self, num_tok: &Token) -> Operand {
        if let TokenKind::Number(sym) = num_tok.kind {
            Operand::Number {
                number: NumericLiteral::unsigned(sym),
                node_id: self.alloc_node_id(num_tok.span),
            }
        } else {
            unreachable!()
        }
    }
}
