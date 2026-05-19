use crate::{
    language::{
        LanguageSymbols,
        instructions::Mnemonic,
        numeric_literals::{NumberSign, NumericLiteral},
    },
    lexer::{Span, Token, TokenKind},
    parser::{
        ir::{AddressOperand, DataDirective, LabelDef, NumberLiteral, SymbolRef},
        types::{DataLine, NodeSpans, TextLine},
    },
};

#[cfg(test)]
use crate::{
    interner::Interner,
    parser::ir::Instruction,
    preprocessor::{LogicalLine, PreprocessedProgram},
};

pub(crate) mod ir;
pub(crate) mod types;

pub(crate) struct Parser<'a> {
    language: &'a LanguageSymbols,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(language: &'a LanguageSymbols) -> Self {
        Self { language }
    }

    #[cfg(test)]
    pub(crate) fn parse_text(
        &self,
        lines: &[LogicalLine],
        node_spans: &mut NodeSpans,
    ) -> Result<Vec<TextLine>, &str> {
        lines
            .iter()
            .map(|line| self.parse_text_line(&line.content, node_spans))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn parse_data(
        &self,
        lines: &[LogicalLine],
        node_spans: &mut NodeSpans,
    ) -> Result<Vec<DataLine>, &str> {
        lines
            .iter()
            .map(|line| self.parse_data_line(&line.content, node_spans))
            .collect()
    }

    pub(crate) fn parse_text_line(
        &self,
        mut line: &[Token],
        node_spans: &mut NodeSpans,
    ) -> Result<TextLine, &str> {
        let line_node_id = node_spans.alloc(Self::span_from_tokens(line));
        let mut label: Option<LabelDef> = None;
        if self.looks_like_label(&line) {
            let (label_def, tail) = self.extract_label(line, node_spans)?;
            label = Some(label_def);
            line = tail;
        }

        let body_line = line;
        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível identificar a instrução")?;

        let TokenKind::Ident(sym) = head.kind else {
            return Err("Não foi possível identificar a instrução");
        };

        let instruction = match sym {
            _ if sym == self.language.instructions.stop => {
                // Caso de instrução com nenhum endereço
                Ok(ir::Instruction::NoOperand {
                    mnemonic: Mnemonic::Stop,
                    node_id: node_spans.alloc(head.span),
                })
            }
            _ if sym == self.language.instructions.copy => {
                // Caso de instrução com dois enderecos
                let mnemonic = self
                    .language
                    .get_mnemonic(&sym)
                    .ok_or("Não foi possível recuperar o Mnemonico da instrução")?;
                let (first, tail) = self.extract_address(tail, node_spans)?;

                let (_, new_tail) = self.extract_operand_separator(tail)?;

                let (second, final_tail) = self.extract_address(new_tail, node_spans)?;
                let node_id =
                    node_spans.alloc(Self::span_from_prefix(body_line, final_tail, head.span));
                Ok(ir::Instruction::TwoOperands {
                    mnemonic: *mnemonic,
                    first,
                    second,
                    node_id,
                })
            }
            _ if self.language.is_instruction(sym) => {
                // Caso de instrução com um endereco
                let mnemonic = self
                    .language
                    .get_mnemonic(&sym)
                    .ok_or("Não foi possível recuperar o Mnemonico da instrução")?;
                let (address, final_tail) = self.extract_address(tail, node_spans)?;
                let node_id =
                    node_spans.alloc(Self::span_from_prefix(body_line, final_tail, head.span));
                Ok(ir::Instruction::OneOperand {
                    mnemonic: *mnemonic,
                    operand: address,
                    node_id,
                })
            }
            _ => Err("Não foi possível identificar a instrução"),
        }?;

        Ok(TextLine {
            label,
            body: instruction,
            node_id: line_node_id,
        })
    }

    pub(crate) fn parse_data_line(
        &self,
        mut line: &[Token],
        node_spans: &mut NodeSpans,
    ) -> Result<DataLine, &str> {
        let line_node_id = node_spans.alloc(Self::span_from_tokens(line));
        let mut label: Option<LabelDef> = None;
        if self.looks_like_label(&line) {
            let (label_def, tail) = self.extract_label(line, node_spans)?;
            label = Some(label_def);
            line = tail;
        }

        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível identificar a diretiva")?;

        let TokenKind::Ident(sym) = head.kind else {
            return Err("Não foi possível identificar a diretiva");
        };

        if !self.language.is_directive(sym) {
            return Err("Não foi possível identificar a diretiva");
        }

        let directive = match sym {
            _ if sym == self.language.data_directives.const_ => {
                let literal = self.extract_number_literal(tail, node_spans)?;
                Ok(DataDirective::Const {
                    value: literal,
                    node_id: node_spans.alloc(head.span),
                })
            }
            _ if sym == self.language.data_directives.space => {
                let amount = self.extract_number_literal(tail, node_spans).ok();
                // Não aceita declaracoes como "SPACE +10" ou "SPACE -10", apenas "SPACE 10"
                if matches!(
                    amount,
                    Some(NumberLiteral {
                        literal: NumericLiteral {
                            sign: Some(_),
                            symbol: _,
                        },
                        ..
                    })
                ) {
                    return Err("A quantidade de espaço deve ser um número literal sem sinal");
                }
                Ok(DataDirective::Space {
                    amount,
                    node_id: node_spans.alloc(head.span),
                })
            }
            _ => Err("Não foi possível identificar a diretiva"),
        }?;

        Ok(DataLine {
            label,
            body: directive,
            node_id: line_node_id,
        })
    }

    fn extract_address<'b>(
        &self,
        line: &'b [Token],
        node_spans: &mut NodeSpans,
    ) -> Result<(AddressOperand, &'b [Token]), &str> {
        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível recuperar o endereço")?;

        let identifier = match head.kind {
            TokenKind::Ident(sym) if !self.language.is_reserved(sym) => Ok(sym),
            _ => Err("Não foi possível recuperar o endereço"),
        }?;

        let base_node_id = node_spans.alloc(head.span);

        if let (Some(sign_tok), Some(num_tok)) = (tail.first(), tail.get(1)) {
            let sign = match sign_tok.kind {
                TokenKind::Plus => Some(NumberSign::Plus),
                TokenKind::Minus => Some(NumberSign::Minus),
                _ => None,
            };

            if let (Some(sign), TokenKind::Number(sym)) = (sign, num_tok.kind) {
                let literal_span = Self::span_between(sign_tok.span, num_tok.span);
                let operand_span = Self::span_between(head.span, num_tok.span);
                let address_operand = AddressOperand::Offset {
                    base: SymbolRef {
                        name: identifier,
                        node_id: base_node_id,
                    },
                    offset: NumberLiteral {
                        literal: NumericLiteral {
                            sign: Some(sign),
                            symbol: sym,
                        },
                        node_id: node_spans.alloc(literal_span),
                    },
                    node_id: node_spans.alloc(operand_span),
                };
                let (_, new_tail) = tail.split_at(2);
                return Ok((address_operand, new_tail));
            }
        }

        Ok((
            AddressOperand::Direct(SymbolRef {
                name: identifier,
                node_id: base_node_id,
            }),
            tail,
        ))
    }

    fn extract_label<'b>(
        &self,
        line: &'b [Token],
        node_spans: &mut NodeSpans,
    ) -> Result<(LabelDef, &'b [Token]), &str> {
        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível recuperar a label")?;
        let TokenKind::Ident(sym) = head.kind else {
            return Err("Não foi possível recuperar a label");
        };
        let label_def = LabelDef {
            name: sym,
            node_id: node_spans.alloc(head.span),
        };

        // Remove TokenKind::Colon
        let (_, remaining) = tail
            .split_first()
            .ok_or("Não foi possível recuperar a label")?;
        Ok((label_def, remaining))
    }

    fn extract_number_literal<'b>(
        &self,
        line: &'b [Token],
        node_spans: &mut NodeSpans,
    ) -> Result<NumberLiteral, &str> {
        match line[..] {
            [
                Token {
                    kind: TokenKind::Number(sym),
                    span,
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: None,
                    symbol: sym,
                },
                node_id: node_spans.alloc(span),
            }),
            [
                Token {
                    kind: TokenKind::Plus,
                    span: sign_span,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    span: number_span,
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: Some(NumberSign::Plus),
                    symbol: sym,
                },
                node_id: node_spans.alloc(Self::span_between(sign_span, number_span)),
            }),
            [
                Token {
                    kind: TokenKind::Minus,
                    span: sign_span,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    span: number_span,
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: Some(NumberSign::Minus),
                    symbol: sym,
                },
                node_id: node_spans.alloc(Self::span_between(sign_span, number_span)),
            }),
            _ => Err("Não foi possível encontrar o valor da constante"),
        }
    }

    fn extract_operand_separator<'b>(&self, line: &'b [Token]) -> Result<((), &'b [Token]), &str> {
        match line.split_first() {
            Some((
                Token {
                    kind: TokenKind::Comma,
                    ..
                },
                tail,
            )) => Ok(((), tail)),
            _ => Err("Não foi possível encontrar o separador de argumentos"),
        }
    }

    fn looks_like_label(&self, line: &[Token]) -> bool {
        matches!(
            line[..],
            [
                Token {
                    kind: TokenKind::Ident(_),
                    ..
                },
                Token {
                    kind: TokenKind::Colon,
                    ..
                },
                ..
            ]
        )
    }

    fn span_from_tokens(tokens: &[Token]) -> Span {
        match (tokens.first(), tokens.last()) {
            (Some(first), Some(last)) => Self::span_between(first.span, last.span),
            _ => Span::default(),
        }
    }

    fn span_from_prefix(line: &[Token], tail: &[Token], fallback: Span) -> Span {
        let consumed_len = line.len().saturating_sub(tail.len());
        if consumed_len == 0 {
            fallback
        } else {
            Self::span_from_tokens(&line[..consumed_len])
        }
    }

    fn span_between(first: Span, last: Span) -> Span {
        let end = last.pos.saturating_add(last.len);

        Span {
            pos: first.pos,
            line: first.line,
            column: first.column,
            len: end.saturating_sub(first.pos),
        }
    }
}

#[cfg(test)]
fn test_span(pos: usize, column: u32, len: usize) -> Span {
    Span {
        pos,
        line: 1,
        column,
        len,
    }
}

#[test]
fn parser_allocates_specific_node_spans_for_text_line_parts() {
    let mut interner = Interner::new();
    let rot = interner.entry("ROT").or_insert();
    let load = interner.entry("LOAD").or_insert();
    let table = interner.entry("TABLE").or_insert();
    let two = interner.entry("2").or_insert();

    let language = LanguageSymbols::new(&mut interner);
    let parser = Parser::new(&language);
    let mut node_spans = NodeSpans::new();

    let line = vec![
        Token::new(TokenKind::Ident(rot), test_span(0, 1, 3)),
        Token::new(TokenKind::Colon, test_span(3, 4, 1)),
        Token::new(TokenKind::Ident(load), test_span(5, 6, 4)),
        Token::new(TokenKind::Ident(table), test_span(10, 11, 5)),
        Token::new(TokenKind::Plus, test_span(16, 17, 1)),
        Token::new(TokenKind::Number(two), test_span(18, 19, 1)),
    ];

    let parsed = parser
        .parse_text_line(&line, &mut node_spans)
        .expect("linha de TEXT deve ser parseada");

    assert_eq!(node_spans.span_of(parsed.node_id), test_span(0, 1, 19));
    assert_eq!(
        node_spans.span_of(parsed.label.as_ref().unwrap().node_id),
        test_span(0, 1, 3)
    );

    let Instruction::OneOperand {
        operand:
            AddressOperand::Offset {
                base,
                offset,
                node_id: operand_node,
            },
        node_id: instruction_node,
        ..
    } = parsed.body
    else {
        panic!("esperava instrução com operando deslocado");
    };

    assert_eq!(node_spans.span_of(instruction_node), test_span(5, 6, 14));
    assert_eq!(node_spans.span_of(base.node_id), test_span(10, 11, 5));
    assert_eq!(node_spans.span_of(offset.node_id), test_span(16, 17, 3));
    assert_eq!(node_spans.span_of(operand_node), test_span(10, 11, 9));
}

#[test]
fn test_func() {
    let mut interner = Interner::new();
    let aux = interner.entry("AUX").or_insert();
    let aux2 = interner.entry("AUX2").or_insert();
    let rot = interner.entry("ROT").or_insert();
    let value10 = interner.entry("10").or_insert();

    let mut language = LanguageSymbols::new(&mut interner);
    let add = language.instructions.add;
    let copy = language.instructions.copy;
    let parser = Parser::new(&mut language);

    let program = PreprocessedProgram {
        text: vec![
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(add),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux2),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Plus,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(rot),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Colon,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(copy),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Plus,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Comma,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux2),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
        ],
        data: vec![],
    };

    let mut node_spans = NodeSpans::new();
    let result = parser
        .parse_text_line(&program.text[0].content, &mut node_spans)
        .expect("Erro ao fazer o parse da linha");

    let Instruction::OneOperand {
        mnemonic,
        operand:
            AddressOperand::Offset {
                base: SymbolRef { name, .. },
                offset:
                    NumberLiteral {
                        literal: NumericLiteral { sign, symbol },
                        ..
                    },
                ..
            },
        ..
    } = result.body
    else {
        panic!(
            "  
            Instrucao não é igual ao esperado
            Esperava OneOperand Offset
            "
        );
    };

    println!(
        "
            [ADD AUX2]
            Label é vazia {:?}
            Instruction: 
            \tMnemonic {:?}
            \tOperand {:?} {:?} {:?} == {:?} {:?} {:?}
        ",
        result.label.is_none(),
        mnemonic,
        name,
        sign,
        symbol,
        aux2,
        NumberSign::Minus,
        value10
    );

    let result2 = parser
        .parse_text_line(&program.text[1].content, &mut node_spans)
        .expect("Deu erro ao parser linha de copy com label");

    let Instruction::TwoOperands {
        mnemonic: mnemonic2,
        first:
            AddressOperand::Offset {
                base: SymbolRef { name, .. },
                offset:
                    NumberLiteral {
                        literal: NumericLiteral { sign, symbol },
                        ..
                    },
                ..
            },
        second: AddressOperand::Direct(SymbolRef {
            name: second_name, ..
        }),
        ..
    } = result2.body
    else {
        panic!(
            "  
            Instrucao não é igual ao esperado
            Esperava TwoOperands Directs
            "
        );
    };

    println!(
        "
            [ROT: COPY AUX AUX2]
            Label ROT {:?} == {:?}
            Instruction: 
            \tMnemonic {:?} == Copy
            \tOperand1 {:?} {:?} {:?}
            \tOperand2 {:?} == {:?}
        ",
        result2.label.unwrap().name,
        rot,
        mnemonic2,
        name,
        sign,
        symbol,
        second_name,
        aux2
    );
}

#[test]
fn test_parse_text() {
    let mut interner = Interner::new();
    let aux = interner.entry("AUX").or_insert();
    let aux2 = interner.entry("AUX2").or_insert();
    let rot = interner.entry("ROT").or_insert();
    let value10 = interner.entry("10").or_insert();

    let mut language = LanguageSymbols::new(&mut interner);
    let add = language.instructions.add;
    let copy = language.instructions.copy;
    let parser = Parser::new(&mut language);

    let program = PreprocessedProgram {
        text: vec![
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(add),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux2),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Plus,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(rot),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Colon,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(copy),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Plus,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Comma,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux2),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
        ],
        data: vec![],
    };

    let mut node_spans = NodeSpans::new();
    let parser_result = parser.parse_text(&program.text, &mut node_spans);

    parser_result.expect("Deu erro no parsing do text");
}

#[test]
fn test_parse_data() {
    let mut interner = Interner::new();
    let aux = interner.entry("AUX").or_insert();
    let label = interner.entry("LABEL").or_insert();
    let value10 = interner.entry("10").or_insert();

    let mut language = LanguageSymbols::new(&mut interner);
    let const_ = language.data_directives.const_;
    let space = language.data_directives.space;
    let parser = Parser::new(&mut language);

    let program = PreprocessedProgram {
        text: vec![],
        data: vec![
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(label),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Colon,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(const_),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Minus,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(aux),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Colon,
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(space),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Number(value10),
                        span: Span::default(),
                    },
                ],
                terminator: Token {
                    kind: TokenKind::NewLine,
                    span: Span::default(),
                },
            },
        ],
    };

    let mut node_spans = NodeSpans::new();
    let parser_result = parser.parse_data(&program.data, &mut node_spans);

    parser_result.expect("Deu erro no parsing do data");
}
