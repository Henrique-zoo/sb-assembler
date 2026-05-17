use std::string::ParseError;

use crate::{
    interner::{Entry, Interner},
    language::{
        self, LanguageSymbols,
        instructions::{self, Mnemonic},
        numeric_literals::{NumberSign, NumericLiteral},
    },
    lexer::{Span, Token, TokenKind},
    parser::{
        ir::{AddressOperand, DataDirective, Instruction, LabelDef, NumberLiteral, SymbolRef},
        types::{DataLine, NodeSpans, ParsedProgram, TextLine},
    },
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

    pub(crate) fn parse(&self, program: PreprocessedProgram) -> Result<ParsedProgram, ParseError> {
        let parsed_text = self.parse_text(&program.text).unwrap_or(Vec::default());
        let parsed_data = self.parse_data(&program.data).unwrap_or(Vec::default());
        Ok(ParsedProgram {
            text: parsed_text,
            data: parsed_data,
            node_spans: NodeSpans::new(),
        })
    }

    pub(crate) fn parse_text(&self, lines: &[LogicalLine]) -> Result<Vec<TextLine>, &str> {
        lines
            .iter()
            .map(|line| self.parse_text_line(&line.content))
            .collect()
    }

    pub(crate) fn parse_data(&self, lines: &[LogicalLine]) -> Result<Vec<DataLine>, &str> {
        lines
            .iter()
            .map(|line| self.parse_data_line(&line.content))
            .collect()
    }

    pub(crate) fn parse_text_line(&self, mut line: &[Token]) -> Result<TextLine, &str> {
        let mut label: Option<LabelDef> = None;
        if self.looks_like_label(&line) {
            let (label_def, tail) = self.extract_label(line)?;
            label = Some(label_def);
            line = tail;
        }

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
                    node_id: ir::NodeId(0),
                })
            }
            _ if sym == self.language.instructions.copy => {
                // Caso de instrução com dois enderecos
                let mnemonic = self
                    .language
                    .get_mnemonic(&sym)
                    .ok_or("Não foi possível recuperar o Mnemonico da instrução")?;
                let (first, tail) = self.extract_address(tail)?;

                let (_, new_tail) = self.extract_operand_separator(tail)?;

                let (second, _) = self.extract_address(new_tail)?;
                Ok(ir::Instruction::TwoOperands {
                    mnemonic: *mnemonic,
                    first,
                    second,
                    node_id: ir::NodeId(0),
                })
            }
            _ if self.language.is_instruction(sym) => {
                // Caso de instrução com um endereco
                let mnemonic = self
                    .language
                    .get_mnemonic(&sym)
                    .ok_or("Não foi possível recuperar o Mnemonico da instrução")?;
                let (address, _) = self.extract_address(tail)?;
                Ok(ir::Instruction::OneOperand {
                    mnemonic: *mnemonic,
                    operand: address,
                    node_id: ir::NodeId(0),
                })
            }
            _ => Err("Não foi possível identificar a instrução"),
        }?;

        Ok(TextLine {
            label,
            body: instruction,
            node_id: ir::NodeId(0),
        })
    }

    pub(crate) fn parse_data_line(&self, mut line: &[Token]) -> Result<DataLine, &str> {
        let mut label: Option<LabelDef> = None;
        if self.looks_like_label(&line) {
            let (label_def, tail) = self.extract_label(line)?;
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
                let literal = self.extract_number_literal(tail)?;
                Ok(DataDirective::Const {
                    value: literal,
                    node_id: ir::NodeId(0),
                })
            }
            _ if sym == self.language.data_directives.space => {
                let amount = self.extract_number_literal(tail).ok();
                if matches!(
                    amount,
                    Some(NumberLiteral {
                        literal: NumericLiteral {
                            sign: Some(_),
                            symbol: _,
                        },
                        node_id: ir::NodeId(0),
                    })
                ) {
                    return Err("A quantidade de espaço deve ser um número literal sem sinal");
                }
                Ok(DataDirective::Space {
                    amount: amount,
                    node_id: ir::NodeId(0),
                })
            }
            _ => Err("Não foi possível identificar a diretiva"),
        }?;

        Ok(DataLine {
            label,
            body: directive,
            node_id: ir::NodeId(0),
        })
    }

    fn extract_address<'b>(
        &self,
        line: &'b [Token],
    ) -> Result<(AddressOperand, &'b [Token]), &str> {
        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível recuperar o endereço")?;

        let identifier = match head.kind {
            TokenKind::Ident(sym) if !self.language.is_reserved(sym) => Ok(sym),
            _ => Err("Não foi possível recuperar o endereço"),
        }?;

        let address_operand = match tail[..] {
            [
                Token {
                    kind: TokenKind::Plus,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    ..
                },
                ..,
            ] => AddressOperand::Offset {
                base: SymbolRef {
                    name: identifier,
                    node_id: ir::NodeId(0),
                },
                offset: NumberLiteral {
                    literal: NumericLiteral {
                        sign: Some(NumberSign::Plus),
                        symbol: sym,
                    },
                    node_id: ir::NodeId(0),
                },
                node_id: ir::NodeId(0),
            },
            [
                Token {
                    kind: TokenKind::Minus,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    ..
                },
                ..,
            ] => AddressOperand::Offset {
                base: SymbolRef {
                    name: identifier,
                    node_id: ir::NodeId(0),
                },
                offset: NumberLiteral {
                    literal: NumericLiteral {
                        sign: Some(NumberSign::Minus),
                        symbol: sym,
                    },
                    node_id: ir::NodeId(0),
                },
                node_id: ir::NodeId(0),
            },
            _ => AddressOperand::Direct(SymbolRef {
                name: identifier,
                node_id: ir::NodeId(0),
            }),
        };

        if matches!(address_operand, AddressOperand::Offset { .. }) {
            let (_, new_tail) = tail.split_at(2);
            return Ok((address_operand, new_tail));
        }

        Ok((address_operand, tail))
    }

    fn extract_label<'b>(&self, line: &'b [Token]) -> Result<(LabelDef, &'b [Token]), &str> {
        let (head, tail) = line
            .split_first()
            .ok_or("Não foi possível recuperar a label")?;
        let TokenKind::Ident(sym) = head.kind else {
            return Err("Não foi possível recuperar a label");
        };
        let label_def = LabelDef {
            name: sym,
            node_id: ir::NodeId(0),
        };

        // Remove TokenKind::Colon
        let (_, remaining) = tail
            .split_first()
            .ok_or("Não foi possível recuperar a label")?;
        Ok((label_def, remaining))
    }

    fn extract_number_literal<'b>(&self, line: &'b [Token]) -> Result<NumberLiteral, &str> {
        match line[..] {
            [
                Token {
                    kind: TokenKind::Number(sym),
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: None,
                    symbol: sym,
                },
                node_id: ir::NodeId(0),
            }),
            [
                Token {
                    kind: TokenKind::Plus,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: Some(NumberSign::Plus),
                    symbol: sym,
                },
                node_id: ir::NodeId(0),
            }),
            [
                Token {
                    kind: TokenKind::Minus,
                    ..
                },
                Token {
                    kind: TokenKind::Number(sym),
                    ..
                },
                ..,
            ] => Ok(NumberLiteral {
                literal: NumericLiteral {
                    sign: Some(NumberSign::Minus),
                    symbol: sym,
                },
                node_id: ir::NodeId(0),
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

    let result = parser
        .parse_text_line(&program.text[0].content)
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
        .parse_text_line(&program.text[1].content)
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

    let parser_result = parser.parse_text(&program.text);

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

    let parser_result = parser.parse_data(&program.data);

    parser_result.expect("Deu erro no parsing do data");
}
