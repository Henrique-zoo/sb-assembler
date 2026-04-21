use super::*;

fn lex_ok(source: &str) -> (Vec<Token>, Interner) {
    let mut interner = Interner::new();
    let tokens = {
        let lexer = Lexer::new(source, &mut interner);
        lexer.collect::<Result<Vec<_>, _>>()
    }
    .expect("lexer should succeed");

    (tokens, interner)
}

fn lex_err(source: &str) -> crate::errors::LexerError {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);

    lexer
        .find_map(Result::err)
        .expect("lexer should produce an error")
}

fn assert_token(token: &Token, kind: TokenKind, span: Span) {
    assert_eq!(token, &Token::new(kind, span));
}

#[test]
fn scan_basic_program() {
    let source = concat!(
        "ROT: INPUT N1\n",
        "COPY N2, N1     ; comentário com utf8\n",
        "N1: SPACE\n",
    );

    let (tokens, mut interner) = lex_ok(source);

    let rot = interner.entry("ROT").or_insert();
    let input = interner.entry("INPUT").or_insert();
    let n1 = interner.entry("N1").or_insert();
    let copy = interner.entry("COPY").or_insert();
    let n2 = interner.entry("N2").or_insert();
    let space = interner.entry("SPACE").or_insert();

    let expected_kinds = vec![
        TokenKind::Ident(rot),
        TokenKind::Colon,
        TokenKind::Ident(input),
        TokenKind::Ident(n1),
        TokenKind::NewLine,
        TokenKind::Ident(copy),
        TokenKind::Ident(n2),
        TokenKind::Comma,
        TokenKind::Ident(n1),
        TokenKind::NewLine,
        TokenKind::Ident(n1),
        TokenKind::Colon,
        TokenKind::Ident(space),
        TokenKind::NewLine,
        TokenKind::Eof,
    ];

    let actual_kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();

    assert_eq!(actual_kinds, expected_kinds);
}

#[test]
fn preserves_spans_across_lines_and_punctuation() {
    let source = "_TMP1: 42,+-\nNEXT";
    let (tokens, mut interner) = lex_ok(source);

    let tmp1 = interner.entry("_TMP1").or_insert();
    let number = interner.entry("42").or_insert();
    let next = interner.entry("NEXT").or_insert();

    assert_eq!(tokens.len(), 9);
    assert_token(
        &tokens[0],
        TokenKind::Ident(tmp1),
        Span {
            pos: 0,
            line: 1,
            column: 1,
            len: 5,
        },
    );
    assert_token(
        &tokens[1],
        TokenKind::Colon,
        Span {
            pos: 5,
            line: 1,
            column: 6,
            len: 1,
        },
    );
    assert_token(
        &tokens[2],
        TokenKind::Number(number),
        Span {
            pos: 7,
            line: 1,
            column: 8,
            len: 2,
        },
    );
    assert_token(
        &tokens[3],
        TokenKind::Comma,
        Span {
            pos: 9,
            line: 1,
            column: 10,
            len: 1,
        },
    );
    assert_token(
        &tokens[4],
        TokenKind::Plus,
        Span {
            pos: 10,
            line: 1,
            column: 11,
            len: 1,
        },
    );
    assert_token(
        &tokens[5],
        TokenKind::Minus,
        Span {
            pos: 11,
            line: 1,
            column: 12,
            len: 1,
        },
    );
    assert_token(
        &tokens[6],
        TokenKind::NewLine,
        Span {
            pos: 12,
            line: 1,
            column: 13,
            len: 1,
        },
    );
    assert_token(
        &tokens[7],
        TokenKind::Ident(next),
        Span {
            pos: 13,
            line: 2,
            column: 1,
            len: 4,
        },
    );
    assert_token(
        &tokens[8],
        TokenKind::Eof,
        Span {
            pos: 17,
            line: 2,
            column: 5,
            len: 0,
        },
    );
}

#[test]
fn reuses_symbols_for_repeated_lexemes() {
    let source = "X X 10 10\n";
    let (tokens, _) = lex_ok(source);

    match (
        &tokens[0].kind,
        &tokens[1].kind,
        &tokens[2].kind,
        &tokens[3].kind,
    ) {
        (
            TokenKind::Ident(first_ident),
            TokenKind::Ident(second_ident),
            TokenKind::Number(first_number),
            TokenKind::Number(second_number),
        ) => {
            assert_eq!(first_ident, second_ident);
            assert_eq!(first_number, second_number);
        }
        other => panic!("unexpected token sequence: {other:?}"),
    }
}

#[test]
fn splits_adjacent_number_and_identifier() {
    let source = "123ABC";
    let (tokens, mut interner) = lex_ok(source);

    let number = interner.entry("123").or_insert();
    let ident = interner.entry("ABC").or_insert();

    let actual_kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        actual_kinds,
        vec![
            TokenKind::Number(number),
            TokenKind::Ident(ident),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_ampersand_tokens_in_parameter_like_sequence() {
    let source = "&A, &B\n";
    let (tokens, mut interner) = lex_ok(source);

    let a = interner.entry("A").or_insert();
    let b = interner.entry("B").or_insert();
    let actual_kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();

    assert_eq!(
        actual_kinds,
        vec![
            TokenKind::Ampersand,
            TokenKind::Ident(a),
            TokenKind::Comma,
            TokenKind::Ampersand,
            TokenKind::Ident(b),
            TokenKind::NewLine,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn preserves_span_for_ampersand_token() {
    let source = "&P";
    let (tokens, mut interner) = lex_ok(source);

    let p = interner.entry("P").or_insert();

    assert_eq!(tokens.len(), 3);
    assert_token(
        &tokens[0],
        TokenKind::Ampersand,
        Span {
            pos: 0,
            line: 1,
            column: 1,
            len: 1,
        },
    );
    assert_token(
        &tokens[1],
        TokenKind::Ident(p),
        Span {
            pos: 1,
            line: 1,
            column: 2,
            len: 1,
        },
    );
    assert_token(
        &tokens[2],
        TokenKind::Eof,
        Span {
            pos: 2,
            line: 1,
            column: 3,
            len: 0,
        },
    );
}

#[test]
fn lexes_hex_and_binary_numbers_with_prefixes() {
    let source = "0x1f 0X2A 0b101 0B11\n";
    let (tokens, mut interner) = lex_ok(source);

    let n1 = interner.entry("0x1f").or_insert();
    let n2 = interner.entry("0X2A").or_insert();
    let n3 = interner.entry("0b101").or_insert();
    let n4 = interner.entry("0B11").or_insert();

    let actual_kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        actual_kinds,
        vec![
            TokenKind::Number(n1),
            TokenKind::Number(n2),
            TokenKind::Number(n3),
            TokenKind::Number(n4),
            TokenKind::NewLine,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn reports_invalid_prefixed_number_without_digits() {
    for source in ["0x", "0X", "0b", "0B"] {
        let error = lex_err(source);

        match &error.kind {
            LexerErrorKind::InvalidNumber(text) => assert_eq!(text, source),
            other => panic!("unexpected error kind: {other:?}"),
        }

        assert_eq!(
            error.span,
            Span {
                pos: 0,
                line: 1,
                column: 1,
                len: 2,
            }
        );
    }
}

#[test]
fn skips_comments_with_unicode_and_emits_newlines() {
    let source = "; comentário λ\nADD\n; fim";
    let (tokens, mut interner) = lex_ok(source);

    let add = interner.entry("ADD").or_insert();
    let actual_kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();

    assert_eq!(
        actual_kinds,
        vec![
            TokenKind::NewLine,
            TokenKind::Ident(add),
            TokenKind::NewLine,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn emits_only_eof_for_empty_or_whitespace_only_input() {
    for source in ["", " \t\r"] {
        let (tokens, _) = lex_ok(source);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }
}

#[test]
fn emits_eof_after_trailing_horizontal_whitespace() {
    let source = "ADD   \t\r";
    let (tokens, mut interner) = lex_ok(source);

    let add = interner.entry("ADD").or_insert();

    assert_eq!(tokens.len(), 2);
    assert_token(
        &tokens[0],
        TokenKind::Ident(add),
        Span {
            pos: 0,
            line: 1,
            column: 1,
            len: 3,
        },
    );
    assert_token(
        &tokens[1],
        TokenKind::Eof,
        Span {
            pos: 8,
            line: 1,
            column: 9,
            len: 0,
        },
    );
}

#[test]
fn reports_invalid_unicode_character() {
    let error = lex_err("á");

    match error.kind {
        LexerErrorKind::InvalidChar(ch) => assert_eq!(ch, 'á'),
        other => panic!("unexpected error kind: {other:?}"),
    }

    assert_eq!(
        error.span,
        Span {
            pos: 0,
            line: 1,
            column: 1,
            len: 'á'.len_utf8(),
        }
    );
}
