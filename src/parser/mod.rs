use std::string::ParseError;

use crate::{
    interner::{Entry, Interner},
    language::{
        self, LanguageSymbols,
        instructions::{self, Mnemonic},
    },
    lexer::{Span, Token, TokenKind},
    parser::{
        ir::LabelDef,
        types::{DataLine, NodeSpans, ParsedProgram, TextLine},
    },
    preprocessor::{LogicalLine, PreprocessedProgram},
};

pub(crate) mod ir;
pub(crate) mod types;

pub(crate) struct Parser<'a> {
    interner: &'a mut Interner,
    language: &'a LanguageSymbols,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(interner: &'a mut Interner, language: &'a LanguageSymbols) -> Self {
        Self { interner, language }
    }

    pub(crate) fn parse(&self, program: PreprocessedProgram) -> Result<ParsedProgram, ParseError> {
        self.looks_like_label(&program.text[0].content);

        Ok(ParsedProgram {
            text: Vec::new(),
            data: Vec::new(),
            node_spans: NodeSpans::new(),
        })
    }

    pub(crate) fn parse_text(&self, lines: &Vec<LogicalLine>) -> Result<Vec<TextLine>, &str> {
        lines
            .iter()
            .map(|line| self.parse_text_line(&line.content))
            .collect()
    }

    pub(crate) fn parse_data(&self, lines: &Vec<LogicalLine>) -> Result<Vec<DataLine>, &str> {
        Err("Não implementado")
    }

    pub(crate) fn parse_text_line(&self, mut line: &[Token]) -> Result<TextLine, &str> {
        let mut label: Option<LabelDef> = None;
        if self.looks_like_label(&line) {
            let (label_def, tail) = self.extract_label(line)?;
            label = Some(label_def);
            line = tail;
        }

        let (head, tail) = line.split_first().ok_or("")?;

        let TokenKind::Ident(sym) = head.kind else {
            return Err("");
        };

        let result = match sym {
            x if x == self.language.instructions.copy => {
                // Caso de instrução com dois enderecos
                Ok("teste")
            }
            x if x == self.language.instructions.stop => {
                // Caso de instrução com nenhum endereço
                Ok("teste")
            }
            x if self.language.is_instruction(sym) => {
                // Caso de instrução com um endereco
                Ok("teste")
            }
            _ => Err("Não foi possível identificar a instrução"),
        };

        Ok(TextLine {
            label,
            body: ir::Instruction::NoOperand {
                mnemonic: Mnemonic::Stop,
                node_id: ir::NodeId(0),
            },
            node_id: ir::NodeId(0),
        })
    }

    pub(crate) fn extract_label<'b>(
        &self,
        line: &'b [Token],
    ) -> Result<(LabelDef, &'b [Token]), &str> {
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

    pub(crate) fn looks_like_label(&self, line: &[Token]) -> bool {
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
    let mut language = LanguageSymbols::new(&mut interner);

    let add = interner.entry("add").or_insert();
    let aux = interner.entry("aux").or_insert();

    let rot = interner.entry("rot").or_insert();
    let cpy = interner.entry("copy").or_insert();
    let aux2 = interner.entry("aux2").or_insert();

    let parser = Parser::new(&mut interner, &mut language);

    let program = PreprocessedProgram {
        text: vec![
            LogicalLine {
                content: vec![
                    Token {
                        kind: TokenKind::Ident(add),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux),
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
                        kind: TokenKind::Ident(cpy),
                        span: Span::default(),
                    },
                    Token {
                        kind: TokenKind::Ident(aux),
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

    let result_first_line = parser.looks_like_label(&program.text[0].content);
    let result_second_line = parser.looks_like_label(&program.text[1].content);

    println!("Result ADD AUX: {:?}", result_first_line);
    println!("Result ROT: COPY AUX AUX2 {:?}", result_second_line);
}
