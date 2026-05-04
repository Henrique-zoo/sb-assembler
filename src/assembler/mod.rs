//! Fachada de alto nível do assembler.
//!
//! Este módulo define o ponto de entrada para processar um programa assembly,
//! encapsulando o estado compartilhado entre estágios (como o `Interner`) e a
//! coordenação do pipeline.
//!
//! Estado atual:
//! - a estrutura base do pipeline está pronta;
//! - o método [`Assembler::process`] já inicializa o lexer e serve como
//!   gancho para os próximos estágios (pré-processamento, parser e emissão).

use crate::{
    errors::{
        DirectiveKind, DirectiveSyntaxErrorKind, EquDirectiveErrorKind, IfDirectiveErrorKind,
        InvalidArgKind, InvalidParamKind, LexerError, LexerErrorKind, MacroCallErrorKind,
        MacroHeaderErrorKind, PreprocessorError, PreprocessorErrorKind,
        SectionDeclarationErrorKind,
    },
    interner::Interner,
    lexer::Lexer,
    preprocessor::Preprocessor,
};

/// Coordenador principal do pipeline de montagem.
///
/// Guarda o fonte original e estruturas auxiliares reutilizadas pelos estágios.
pub struct Assembler<'a> {
    /// Código-fonte de entrada.
    source: &'a str,
    /// Tabela de internamento compartilhada pelos estágios.
    interner: Interner,
}

impl<'a> Assembler<'a> {
    /// Cria uma instância de assembler para um fonte específico.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            interner: Interner::new(),
        }
    }

    /// Executa o pipeline de montagem.
    ///
    /// Fluxo atual:
    /// 1. análise léxica (`Lexer`);
    /// 2. pré-processamento (`Preprocessor`);
    ///
    /// Observação:
    /// - por enquanto, o método apenas executa os estágios e consome os
    ///   diagnósticos produzidos; ainda não há retorno estruturado para o
    ///   chamador.
    pub fn process(mut self) {
        let tokens = {
            let lexer = Lexer::new(self.source, &mut self.interner);
            match lexer.collect::<Result<Vec<_>, _>>() {
                Ok(tokens) => tokens,
                Err(err) => {
                    Self::touch_lexer_error(&err);
                    return;
                }
            }
        };

        let mut preprocessor = Preprocessor::new(&mut self.interner);

        if let Err(errors) = preprocessor.process(tokens, &mut self.interner) {
            for err in &errors {
                Self::touch_preprocessor_error(err);
            }
        }
    }

    /// Lê explicitamente os campos de erro léxico para diagnóstico interno.
    fn touch_lexer_error(err: &LexerError) {
        match &err.kind {
            LexerErrorKind::InvalidChar(ch) => {
                let _ = ch.len_utf8();
            }
            LexerErrorKind::InvalidIdentifier(text) | LexerErrorKind::InvalidNumber(text) => {
                let _ = text.len();
            }
        }

        let _ = err.span;
    }

    /// Lê explicitamente os campos de erro de pré-processamento para
    /// diagnóstico interno.
    fn touch_preprocessor_error(err: &PreprocessorError) {
        match &err.kind {
            PreprocessorErrorKind::UnexpectedEndMacro
            | PreprocessorErrorKind::UnterminatedMacro
            | PreprocessorErrorKind::InvalidDirective => {}

            PreprocessorErrorKind::MacroAlreadyDefined(sym) => {
                let _ = *sym;
            }

            PreprocessorErrorKind::WrongArgCount { expected, found } => {
                let _ = (*expected, *found);
            }

            PreprocessorErrorKind::InvalidDirectiveSyntax(kind) => match kind {
                DirectiveSyntaxErrorKind::TrailingTokens { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
                DirectiveSyntaxErrorKind::MissingToken {
                    directive,
                    token_missed,
                } => {
                    let _ = *token_missed;
                    match directive {
                        DirectiveKind::MacroHeader
                        | DirectiveKind::EndMacro
                        | DirectiveKind::Equ
                        | DirectiveKind::If
                        | DirectiveKind::Section
                        | DirectiveKind::MacroCall => {}
                    }
                }
                DirectiveSyntaxErrorKind::InvalidDeclaration { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive,
                    expected_token,
                } => {
                    let _ = *expected_token;
                    match directive {
                        DirectiveKind::MacroHeader
                        | DirectiveKind::EndMacro
                        | DirectiveKind::Equ
                        | DirectiveKind::If
                        | DirectiveKind::Section
                        | DirectiveKind::MacroCall => {}
                    }
                }
                DirectiveSyntaxErrorKind::ForbiddenToken { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
            },

            PreprocessorErrorKind::InvalidSectionDeclaration(kind) => match kind {
                SectionDeclarationErrorKind::MissingSectionKeyword => {}
            },

            PreprocessorErrorKind::InvalidMacroHeader(kind) => match kind {
                MacroHeaderErrorKind::MissingColon | MacroHeaderErrorKind::InvalidLabel => {}
                MacroHeaderErrorKind::InvalidParam(param_kind) => match param_kind {
                    InvalidParamKind::InvalidParamIdent
                    | InvalidParamKind::NoAmpersand
                    | InvalidParamKind::UnexpectedComma => {}
                },
            },

            PreprocessorErrorKind::InvalidMacroCall(kind) => match kind {
                MacroCallErrorKind::UndefinedMacro | MacroCallErrorKind::MissingArguments => {}
                MacroCallErrorKind::InvalidArg(arg_kind) => match arg_kind {
                    InvalidArgKind::InvalidArgIdent | InvalidArgKind::UnexpectedComma => {}
                },
            },

            PreprocessorErrorKind::InvalidIfDirective(kind) => match kind {
                IfDirectiveErrorKind::MissingCondition
                | IfDirectiveErrorKind::InvalidConditionType => {}
            },

            PreprocessorErrorKind::InvalidEquDirective(kind) => match kind {
                EquDirectiveErrorKind::InvalidValueType => {}
            },
        }

        let _ = err.span;
    }
}
