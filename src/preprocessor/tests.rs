use super::*;

use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, EquDirectiveSemanticErrorKind,
        EquDirectiveSyntaticErrorKind, ExpectedToken, IfDirectiveSemanticErrorKind,
        IfDirectiveSyntaticErrorKind, InvalidArgKind, MacroCallSemanticErrorKind,
        MacroCallSyntaticErrorKind, PreprocessorError, PreprocessorErrorKind,
    },
    interner::Interner,
    language::LanguageSymbols,
    lexer::{Lexer, Token, TokenKind},
};

fn lex_ok(source: &str, interner: &mut Interner) -> Vec<Token> {
    let lexer = Lexer::new(source, interner);
    lexer
        .collect::<Result<Vec<_>, _>>()
        .expect("lexer should succeed")
}

fn preprocess_ok(source: &str) -> (PreprocessedProgram, Interner) {
    let mut interner = Interner::new();
    let language_symbols = LanguageSymbols::new(&mut interner);
    let tokens = lex_ok(source, &mut interner);
    let mut preprocessor = Preprocessor::new(&language_symbols);

    let lines = preprocessor
        .process(tokens, &mut interner)
        .expect("preprocessor should succeed");

    (lines, interner)
}

fn preprocess_err(source: &str) -> Vec<PreprocessorError> {
    let mut interner = Interner::new();
    let language_symbols = LanguageSymbols::new(&mut interner);
    let tokens = lex_ok(source, &mut interner);
    let mut preprocessor = Preprocessor::new(&language_symbols);

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
    }
}

fn render_preprocessor_lines(lines: &[LogicalLine], interner: &Interner) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
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

fn render_preprocessed_section(
    section_name: &str,
    lines: &[LogicalLine],
    interner: &Interner,
) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!(
            "SECTION {section_name}\n{}",
            render_preprocessor_output(lines, interner)
        )
    }
}

fn render_preprocessed_program(program: &PreprocessedProgram, interner: &Interner) -> String {
    [
        render_preprocessed_section("TEXT", &program.text, interner),
        render_preprocessed_section("DATA", &program.data, interner),
    ]
    .into_iter()
    .filter(|section| !section.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
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
fn does_not_detect_labeled_instruction_as_macro_call() {
    let mut interner = Interner::new();
    let language_symbols = LanguageSymbols::new(&mut interner);
    let tokens = lex_ok("LABEL: ADD VALUE\n", &mut interner);
    let line = &tokens[..tokens.len() - 1];
    let preprocessor = Preprocessor::new(&language_symbols);

    assert!(!preprocessor.looks_like_macro_call(line));
}

#[test]
fn preserves_labeled_instruction_in_text_section() {
    let source = concat!("SECTION TEXT\n", "LABEL: ADD VALUE\n");

    let (program, interner) = preprocess_ok(source);
    let rendered_text = render_preprocessor_lines(&program.text, &interner);

    assert_eq!(rendered_text, vec!["LABEL : ADD VALUE"]);
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

    let (program, interner) = preprocess_ok(source);
    let rendered_text = render_preprocessor_lines(&program.text, &interner);
    let rendered_data = render_preprocessor_lines(&program.data, &interner);
    let rendered_output = render_preprocessed_program(&program, &interner);
    print_preprocessor_case(
        "preprocesses_full_program_with_all_valid_directive_forms",
        source,
        &rendered_output,
    );

    assert!(rendered_output.starts_with("SECTION TEXT\n"));
    assert!(rendered_output.contains("\nSECTION DATA\n"));

    assert_eq!(
        rendered_text,
        vec![
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
        ]
    );

    assert_eq!(
        rendered_data,
        vec!["COUNTER SPACE", "TMP SPACE", "A SPACE", "B SPACE"]
    );
}

#[test]
fn accepts_labeled_endmacro_terminator() {
    let source = concat!(
        "WRAP: MACRO &VALUE\n",
        "LOAD &VALUE\n",
        "CLOSE:\n",
        "ENDMACRO\n",
        "SECTION TEXT\n",
        "WRAP N\n",
        "OUTPUT N\n",
    );

    let (program, interner) = preprocess_ok(source);
    let rendered_text = render_preprocessor_lines(&program.text, &interner);

    assert_eq!(rendered_text, vec!["LOAD N", "CLOSE :", "OUTPUT N"]);
}

#[test]
fn replaces_equ_aliases_only_in_data_directive_operands() {
    let source = concat!(
        "COUNT EQU 3\n",
        "POS EQU +2\n",
        "NEG EQU -1\n",
        "SECTION DATA\n",
        "VALUE: CONST COUNT\n",
        "BUFFER: SPACE COUNT\n",
        "POSVAL: CONST POS\n",
        "NEGVAL: CONST NEG\n",
        "SECTION TEXT\n",
        "LOAD COUNT\n",
    );

    let (program, interner) = preprocess_ok(source);
    let rendered_data = render_preprocessor_lines(&program.data, &interner);
    let rendered_text = render_preprocessor_lines(&program.text, &interner);

    assert_eq!(
        rendered_data,
        vec![
            "VALUE : CONST 3",
            "BUFFER : SPACE 3",
            "POSVAL : CONST + 2",
            "NEGVAL : CONST - 1",
        ]
    );
    assert_eq!(rendered_text, vec!["LOAD COUNT"]);
}

#[test]
fn reports_undefined_equ_alias_in_data_directive_operand() {
    let source = concat!(
        "SECTION DATA\n",
        "VALUE: CONST MISSING\n",
        "BUFFER: SPACE MISSING\n",
    );

    let errors = preprocess_err(source);
    let rendered_output = render_preprocessor_errors(&errors);
    print_preprocessor_case(
        "reports_undefined_equ_alias_in_data_directive_operand",
        source,
        &rendered_output,
    );

    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|err| {
        matches!(
            err.kind,
            PreprocessorErrorKind::InvalidEquDirectiveSemantic(
                EquDirectiveSemanticErrorKind::UndefinedSymbol { .. }
            )
        )
    }));
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
                    expected: ExpectedToken::Exact(TokenKind::Colon),
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

#[test]
fn reports_unterminated_macro_when_another_macro_starts_before_endmacro() {
    let source = concat!(
        "BROKEN: MACRO &A\n",
        "LOAD &A\n",
        "NEXT: MACRO &B\n",
        "STORE &B\n",
        "ENDMACRO\n",
        "SECTION TEXT\n",
        "NEXT VALUE\n",
    );

    let errors = preprocess_err(source);
    let rendered_output = render_preprocessor_errors(&errors);
    print_preprocessor_case(
        "reports_unterminated_macro_when_another_macro_starts_before_endmacro",
        source,
        &rendered_output,
    );

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        PreprocessorErrorKind::UnterminatedMacro
    ));
    assert_eq!(errors[0].span.line, 3);
    assert_eq!(errors[0].span.column, 1);
}
