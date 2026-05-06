use super::*;

use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, EquDirectiveSyntaticErrorKind,
        IfDirectiveSemanticErrorKind, IfDirectiveSyntaticErrorKind, InvalidArgKind,
        MacroCallSemanticErrorKind, MacroCallSyntaticErrorKind, PreprocessorError,
        PreprocessorErrorKind,
    },
    interner::Interner,
    language::KeywordTable,
    lexer::{Lexer, Token, TokenKind},
};

fn lex_ok(source: &str, interner: &mut Interner) -> Vec<Token> {
    let lexer = Lexer::new(source, interner);
    lexer
        .collect::<Result<Vec<_>, _>>()
        .expect("lexer should succeed")
}

fn preprocess_ok(source: &str) -> (Vec<LogicalLine>, Interner) {
    let mut interner = Interner::new();
    let keyword_table = KeywordTable::new(&mut interner);
    let tokens = lex_ok(source, &mut interner);
    let mut preprocessor = Preprocessor::new(&mut interner, keyword_table);

    let lines = preprocessor
        .process(tokens, &mut interner)
        .expect("preprocessor should succeed");

    (lines, interner)
}

fn preprocess_err(source: &str) -> Vec<PreprocessorError> {
    let mut interner = Interner::new();
    let keyword_table = KeywordTable::new(&mut interner);
    let tokens = lex_ok(source, &mut interner);
    let mut preprocessor = Preprocessor::new(&mut interner, keyword_table);

    preprocessor
        .process(tokens, &mut interner)
        .expect_err("preprocessor should fail")
}

fn print_preprocessor_case(case_name: &str, source: &str, output: &str) {
    println!("\n=== {case_name} ===\nEntrada:\n{source}\nSaida do preprocessor:\n{output}\n");
}

fn token_to_text(token: &Token, interner: &Interner) -> String {
    match token.kind {
        TokenKind::Ident(sym) | TokenKind::Number(sym) => interner
            .get_str(sym)
            .expect("symbol should be interned")
            .to_owned(),
        TokenKind::Ampersand => "&".to_owned(),
        TokenKind::Comma => ",".to_owned(),
        TokenKind::Colon => ":".to_owned(),
        TokenKind::Plus => "+".to_owned(),
        TokenKind::Minus => "-".to_owned(),
        TokenKind::NewLine => "\\n".to_owned(),
        TokenKind::Eof => "<EOF>".to_owned(),
    }
}

fn render_preprocessor_lines(lines: &[LogicalLine], interner: &Interner) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if matches!(line.terminator.kind, TokenKind::Eof) {
                return "<EOF>".to_owned();
            }

            line.content
                .iter()
                .map(|token| token_to_text(token, interner))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn render_preprocessor_output(lines: &[LogicalLine], interner: &Interner) -> String {
    render_preprocessor_lines(lines, interner).join("\n")
}

fn render_preprocessor_errors(errors: &[PreprocessorError]) -> String {
    errors
        .iter()
        .map(|err| format!("kind={:?} span={:?}", err.kind, err.span))
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_error(
    errors: &[PreprocessorError],
    predicate: impl Fn(&PreprocessorErrorKind) -> bool,
) -> bool {
    errors.iter().any(|err| predicate(&err.kind))
}

#[test]
fn preprocesses_full_program_with_all_valid_directive_forms() {
    let source = concat!(
        "NOARG: MACRO\n",
        "LOAD ZERO\n",
        "STORE COUNTER\n",
        "ENDMACRO\n",
        "PUSHCONST: MACRO &DST, &VAL\n",
        "LOAD &VAL\n",
        "STORE &DST\n",
        "ENDMACRO\n",
        "SWAP: MACRO &A, &B, &TMP\n",
        "LOAD &A\n",
        "STORE &TMP\n",
        "LOAD &B\n",
        "STORE &A\n",
        "LOAD &TMP\n",
        "STORE &B\n",
        "ENDMACRO\n",
        "ZERO EQU 0\n",
        "ONE EQU +1\n",
        "NEGONE EQU -1\n",
        "HEXVAL EQU 0X0A\n",
        "BINVAL EQU 0B11\n",
        "ALIAS EQU HEXVAL\n",
        "NEG_ALIAS EQU NEGONE\n",
        "SECTION DATA\n",
        "COUNTER SPACE\n",
        "TMP SPACE\n",
        "A SPACE\n",
        "B SPACE\n",
        "SECTION TEXT\n",
        "NOARG\n",
        "PUSHCONST COUNTER, ONE\n",
        "PUSHCONST COUNTER, +2\n",
        "PUSHCONST COUNTER, -3\n",
        "PUSHCONST COUNTER, 5\n",
        "SWAP A, B, TMP\n",
        "IF ONE\n",
        "LOAD COUNTER\n",
        "IF ZERO\n",
        "SUB COUNTER\n",
        "IF +1\n",
        "ADD COUNTER\n",
        "IF -1\n",
        "STORE COUNTER\n",
        "IF ALIAS\n",
        "DIV COUNTER\n",
        "IF NEG_ALIAS\n",
        "MULT COUNTER\n",
        "IF 0X0\n",
        "JMP END\n",
        "IF 0B1\n",
        "SUB COUNTER\n",
        "STOP\n",
    );

    let (lines, interner) = preprocess_ok(source);
    let rendered = render_preprocessor_lines(&lines, &interner);
    let rendered_output = render_preprocessor_output(&lines, &interner);
    print_preprocessor_case(
        "preprocesses_full_program_with_all_valid_directive_forms",
        source,
        &rendered_output,
    );

    assert_eq!(
        rendered,
        vec![
            "SECTION TEXT",
            "LOAD ZERO",
            "STORE COUNTER",
            "LOAD ONE",
            "STORE COUNTER",
            "LOAD + 2",
            "STORE COUNTER",
            "LOAD - 3",
            "STORE COUNTER",
            "LOAD 5",
            "STORE COUNTER",
            "LOAD A",
            "STORE TMP",
            "LOAD B",
            "STORE A",
            "LOAD TMP",
            "STORE B",
            "LOAD COUNTER",
            "ADD COUNTER",
            "STORE COUNTER",
            "DIV COUNTER",
            "MULT COUNTER",
            "SUB COUNTER",
            "STOP",
            "SECTION DATA",
            "COUNTER SPACE",
            "TMP SPACE",
            "A SPACE",
            "B SPACE",
            "<EOF>",
        ]
    );
}

#[test]
fn preprocesses_full_program_and_accumulates_mixed_directive_errors() {
    let source = concat!(
        "BADMACRO MACRO &A\n",
        "BROKENEND: MACRO &A\n",
        "LOAD &A\n",
        "ENDMACRO EXTRA\n",
        "PAIR: MACRO &A, &B\n",
        "LOAD &A\n",
        "ADD &B\n",
        "ENDMACRO\n",
        "FLAG EQU +1\n",
        "ALIAS EQU FLAG\n",
        "BROKEN_COLON: EQU 10\n",
        "BROKEN_VALUE EQU +\n",
        "TEXT\n",
        "SECTION TEXT\n",
        "IF\n",
        "LOAD A\n",
        "IF UNKNOWN\n",
        "LOAD B\n",
        "PAIR A\n",
        "PAIR A,\n",
        "UNDEF A\n",
        "ENDMACRO\n",
        "IF 1\n",
        "STOP\n",
        "SECTION DATA\n",
        "VALUE SPACE\n",
    );

    let errors = preprocess_err(source);
    let rendered_output = render_preprocessor_errors(&errors);
    print_preprocessor_case(
        "preprocesses_full_program_and_accumulates_mixed_directive_errors",
        source,
        &rendered_output,
    );

    assert_eq!(errors.len(), 11);

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidDirectiveSyntax(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::MacroHeader,
                    expected_token: TokenKind::Colon,
                }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidDirectiveSyntax(
                DirectiveSyntaxErrorKind::TrailingTokens {
                    directive: DirectiveKind::EndMacro,
                }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidDirectiveSyntax(
                DirectiveSyntaxErrorKind::ForbiddenToken {
                    directive: DirectiveKind::Equ,
                }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidEquDirectiveSyntatic(
                EquDirectiveSyntaticErrorKind::InvalidValueType
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidDirectiveSyntax(
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive: DirectiveKind::Section,
                    ..
                }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidIfDirectiveSyntatic(
                IfDirectiveSyntaticErrorKind::MissingCondition
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidIfDirectiveSemantic(
                IfDirectiveSemanticErrorKind::UndefinedIdentifier { .. }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidMacroCallSemantic(
                MacroCallSemanticErrorKind::WrongArgCount {
                    expected: 2,
                    found: 1,
                }
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidMacroCallSyntatic(
                MacroCallSyntaticErrorKind::InvalidArg(InvalidArgKind::UnexpectedComma)
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(
            kind,
            PreprocessorErrorKind::InvalidMacroCallSemantic(
                MacroCallSemanticErrorKind::UndefinedMacro
            )
        )
    }));

    assert!(contains_error(&errors, |kind| {
        matches!(kind, PreprocessorErrorKind::UnexpectedEndMacro)
    }));
}

#[test]
fn reports_unterminated_macro_in_complex_preprocessor_flow() {
    let source = concat!(
        "COUNT EQU 1\n",
        "BROKEN: MACRO &A, &B\n",
        "LOAD &A\n",
        "IF COUNT\n",
        "ADD &B\n",
    );

    let errors = preprocess_err(source);
    let rendered_output = render_preprocessor_errors(&errors);
    print_preprocessor_case(
        "reports_unterminated_macro_in_complex_preprocessor_flow",
        source,
        &rendered_output,
    );

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        PreprocessorErrorKind::UnterminatedMacro
    ));
}
