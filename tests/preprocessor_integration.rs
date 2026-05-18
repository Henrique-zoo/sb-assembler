#![allow(dead_code, unused_imports)]

mod assembler {
    pub type SignedWord = i16;
    pub type Word = u16;
}
#[path = "../src/errors/mod.rs"]
mod errors;
#[path = "../src/interner/mod.rs"]
mod interner;
#[path = "../src/language/mod.rs"]
mod language;
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[path = "../src/preprocessor/mod.rs"]
mod preprocessor;

use interner::Interner;
use language::LanguageSymbols;
use lexer::{Lexer, Token, TokenKind};
use preprocessor::{LogicalLine, PreprocessedProgram, Preprocessor};

fn token_to_text(token: &Token, interner: &Interner) -> String {
    match token.kind {
        TokenKind::Ident(sym) | TokenKind::Number(sym) => interner
            .get_str(sym)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("<sym:{sym}>")),
        TokenKind::Ampersand => "&".to_owned(),
        TokenKind::Comma => ",".to_owned(),
        TokenKind::Colon => ":".to_owned(),
        TokenKind::Plus => "+".to_owned(),
        TokenKind::Minus => "-".to_owned(),
        TokenKind::NewLine => "\\n".to_owned(),
    }
}

fn render_lexer_output(tokens: &[Token], interner: &Interner) -> String {
    let mut out = String::new();
    let mut current_line = Vec::new();

    for token in tokens {
        match token.kind {
            TokenKind::NewLine => {
                out.push_str(&current_line.join(" "));
                out.push('\n');
                current_line.clear();
            }
            _ => current_line.push(token_to_text(token, interner)),
        }
    }

    if !current_line.is_empty() {
        out.push_str(&current_line.join(" "));
    }

    out
}

fn render_preprocessor_output(lines: &[LogicalLine], interner: &Interner) -> String {
    let mut out = String::new();

    for line in lines {
        let line_text = line
            .content
            .iter()
            .map(|token| token_to_text(token, interner))
            .collect::<Vec<_>>()
            .join(" ");

        out.push_str(&line_text);

        match line.terminator.kind {
            TokenKind::NewLine => out.push('\n'),
            _ => {}
        }
    }

    out
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

fn render_preprocessed_output(program: &PreprocessedProgram, interner: &Interner) -> String {
    [
        render_preprocessed_section("TEXT", &program.text, interner),
        render_preprocessed_section("DATA", &program.data, interner),
    ]
    .into_iter()
    .filter(|section| !section.is_empty())
    .collect::<Vec<_>>()
    .join("")
}

#[test]
fn lexer_and_preprocessor_integration() {
    let source = r#"; ------------------------------------------------------------
; CENARIO: preprocessor completo com MACRO, EQU, IF e seções
; ------------------------------------------------------------

ACCUM: MACRO &DST, &SRC, &TMP
LOAD &TMP
ADD &TMP, &SRC
STORE &DST
ENDMACRO

CLEAR: MACRO &DST
LOAD ZERO
STORE &DST
ENDMACRO

ZERO EQU 0
ONE EQU 1
TRUE EQU ONE
ENABLE_LOG EQU TRUE

SECTION DATA
INPUT SPACE
AUX SPACE
RESULT SPACE
LOGPTR SPACE

SECTION TEXT
ACCUM RESULT, INPUT, AUX
IF ENABLE_LOG
STORE LOGPTR
IF 0
ADD INPUT, ONE
CLEAR AUX
"#;

    let mut interner = Interner::new();
    let language_symbols = LanguageSymbols::new(&mut interner);

    let tokens = {
        let lexer = Lexer::new(source, &mut interner);
        lexer.collect::<Result<Vec<_>, _>>()
    }
    .expect("lexer falhou no cenário de teste");

    let lexer_output = render_lexer_output(&tokens, &interner);
    println!("===== LEXER OUTPUT =====\n{lexer_output}");

    let mut preprocessor = Preprocessor::new(&language_symbols);
    let preprocessed = preprocessor
        .process(tokens, &mut interner)
        .expect("preprocessor falhou no cenário de teste");

    let preprocessor_output = render_preprocessed_output(&preprocessed, &interner);
    println!("===== PREPROCESSOR OUTPUT =====\n{preprocessor_output}");

    assert!(
        preprocessor_output.starts_with("SECTION TEXT\n"),
        "output do preprocessor deve identificar a seção TEXT"
    );
    assert!(
        preprocessor_output.contains("SECTION DATA\n"),
        "output do preprocessor deve identificar a seção DATA"
    );
    assert!(
        preprocessor_output.contains("LOAD AUX"),
        "macro ACCUM não expandiu corretamente"
    );
    assert!(
        preprocessor_output.contains("ADD AUX , INPUT"),
        "macro ACCUM não expandiu corretamente"
    );
    assert!(
        preprocessor_output.contains("STORE RESULT"),
        "macro ACCUM não expandiu corretamente"
    );
    assert!(
        preprocessor_output.contains("STORE LOGPTR"),
        "IF ENABLE_LOG deveria incluir a linha seguinte"
    );
    assert!(
        !preprocessor_output.contains("ADD INPUT , ONE"),
        "IF 0 deveria descartar a linha seguinte"
    );
    assert!(
        preprocessor_output.contains("INPUT SPACE")
            && preprocessor_output.contains("AUX SPACE")
            && preprocessor_output.contains("RESULT SPACE")
            && preprocessor_output.contains("LOGPTR SPACE"),
        "seção DATA não foi preservada no output"
    );
    assert!(
        preprocessor_output.ends_with('\n'),
        "output do preprocessor deve preservar a quebra final"
    );
}
