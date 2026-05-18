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
        DirectiveKind, DirectiveSyntaxErrorKind, EquDirectiveSemanticErrorKind,
        EquDirectiveSyntaticErrorKind, ExpectedToken, IfDirectiveSemanticErrorKind,
        IfDirectiveSyntaticErrorKind, InvalidArgKind, InvalidParamKind, LexerError, LexerErrorKind,
        MacroCallSemanticErrorKind, MacroCallSyntaticErrorKind, MacroHeaderErrorKind,
        PreprocessorError, PreprocessorErrorKind,
    },
    interner::Interner,
    language::LanguageSymbols,
    lexer::Lexer,
    preprocessor::Preprocessor,
};

/// Tipo base de palavra da arquitetura alvo (16 bits).
///
/// Esse alias representa o inteiro "natural" da ISA, independente do `usize`
/// da máquina host que executa o assembler.
pub type Word = u16;

/// Tipo base de palavra com sinal da arquitetura alvo (16 bits).
///
/// Útil para estágios que precisam representar valores assinados de 16 bits
/// sem depender do tamanho de inteiros da máquina host.
pub type SignedWord = i16;

/// Coordenador principal do pipeline de montagem.
///
/// Guarda o fonte original e estruturas auxiliares reutilizadas pelos estágios.
pub struct Assembler<'a> {
    /// Código-fonte de entrada.
    source: &'a str,
    /// Tabela de internamento compartilhada pelos estágios.
    interner: Interner,
    /// Vocabulário internado da linguagem.
    language_symbols: LanguageSymbols,
}

impl<'a> Assembler<'a> {
    /// Cria uma instância de assembler para um fonte específico.
    pub fn new(source: &'a str) -> Self {
        let mut interner = Interner::new();
        let language_symbols = LanguageSymbols::new(&mut interner);

        Self {
            source,
            interner,
            language_symbols,
        }
    }

    /// Executa o pipeline de montagem.
    ///
    /// Fluxo atual:
    /// 1. análise léxica (`Lexer`);
    /// 2. pré-processamento (`Preprocessor`);
    ///
    /// Normalização de case:
    /// - antes da análise léxica, o fonte é convertido para
    ///   `ASCII uppercase` (`to_ascii_uppercase`), de modo que a linguagem se
    ///   comporte como case-insensitive para o pipeline principal.
    ///
    /// Observação:
    /// - por enquanto, o método apenas executa os estágios e consome os
    ///   diagnósticos produzidos; ainda não há retorno estruturado para o
    ///   chamador.
    pub fn process(mut self) {
        let normalized_source = self.source.to_ascii_uppercase();

        let tokens = {
            let lexer = Lexer::new(normalized_source.as_str(), &mut self.interner);
            match lexer.collect::<Result<Vec<_>, _>>() {
                Ok(tokens) => tokens,
                Err(err) => {
                    Self::touch_lexer_error(&err);
                    return;
                }
            }
        };

        let mut preprocessor = Preprocessor::new(&self.language_symbols);

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
            PreprocessorErrorKind::UnknownLineSyntax => {}

            PreprocessorErrorKind::UnexpectedEndMacro
            | PreprocessorErrorKind::UnterminatedMacro => {}

            PreprocessorErrorKind::MacroAlreadyDefined(sym) => {
                let _ = *sym;
            }

            PreprocessorErrorKind::InvalidDirectiveSyntax(kind) => match kind {
                DirectiveSyntaxErrorKind::TrailingTokens { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::MacroBody
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
                DirectiveSyntaxErrorKind::MissingToken {
                    directive,
                    expected,
                } => {
                    Self::touch_expected_token(expected);
                    match directive {
                        DirectiveKind::MacroHeader
                        | DirectiveKind::MacroBody
                        | DirectiveKind::EndMacro
                        | DirectiveKind::Equ
                        | DirectiveKind::If
                        | DirectiveKind::Section
                        | DirectiveKind::MacroCall => {}
                    }
                }
                DirectiveSyntaxErrorKind::InvalidDeclaration { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::MacroBody
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
                DirectiveSyntaxErrorKind::UnexpectedToken {
                    directive,
                    expected,
                } => {
                    Self::touch_expected_token(expected);
                    match directive {
                        DirectiveKind::MacroHeader
                        | DirectiveKind::MacroBody
                        | DirectiveKind::EndMacro
                        | DirectiveKind::Equ
                        | DirectiveKind::If
                        | DirectiveKind::Section
                        | DirectiveKind::MacroCall => {}
                    }
                }
                DirectiveSyntaxErrorKind::ForbiddenToken { directive } => match directive {
                    DirectiveKind::MacroHeader
                    | DirectiveKind::MacroBody
                    | DirectiveKind::EndMacro
                    | DirectiveKind::Equ
                    | DirectiveKind::If
                    | DirectiveKind::Section
                    | DirectiveKind::MacroCall => {}
                },
            },

            PreprocessorErrorKind::InvalidMacroHeader(kind) => match kind {
                MacroHeaderErrorKind::InvalidLabel => {}
                MacroHeaderErrorKind::InvalidParam(param_kind) => match param_kind {
                    InvalidParamKind::InvalidParamIdent
                    | InvalidParamKind::NoAmpersand
                    | InvalidParamKind::UnexpectedComma => {}
                },
            },

            PreprocessorErrorKind::InvalidMacroCallSyntatic(kind) => match kind {
                MacroCallSyntaticErrorKind::InvalidArg(arg_kind) => match arg_kind {
                    InvalidArgKind::InvalidArgIdent | InvalidArgKind::UnexpectedComma => {}
                },
            },

            PreprocessorErrorKind::InvalidMacroCallSemantic(kind) => match kind {
                MacroCallSemanticErrorKind::UndefinedMacro
                | MacroCallSemanticErrorKind::MissingArguments => {}
                MacroCallSemanticErrorKind::WrongArgCount { expected, found } => {
                    let _ = (*expected, *found);
                }
            },

            PreprocessorErrorKind::InvalidIfDirectiveSyntatic(kind) => match kind {
                IfDirectiveSyntaticErrorKind::MissingCondition
                | IfDirectiveSyntaticErrorKind::InvalidConditionType => {}
            },

            PreprocessorErrorKind::InvalidIfDirectiveSemantic(kind) => match kind {
                IfDirectiveSemanticErrorKind::ConditionNumberOverflow { value: _ } => {}
                IfDirectiveSemanticErrorKind::InvalidConditionNumber { value: _ } => {}
                IfDirectiveSemanticErrorKind::InvalidConditionIdentifier { ident: _, value: _ } => {
                }
                IfDirectiveSemanticErrorKind::UndefinedIdentifier { ident: _ } => {}
                IfDirectiveSemanticErrorKind::MissingNextLine => {}
            },

            PreprocessorErrorKind::InvalidEquDirectiveSyntatic(kind) => match kind {
                EquDirectiveSyntaticErrorKind::InvalidValueType => {}
            },

            PreprocessorErrorKind::InvalidEquDirectiveSemantic(kind) => match kind {
                EquDirectiveSemanticErrorKind::UndefinedSymbol { symbol: _ } => {}
                EquDirectiveSemanticErrorKind::InvalidValueNumber { value: _ } => {}
                EquDirectiveSemanticErrorKind::ValueNumberOverflow { value: _ } => {}
            },
        }

        let _ = err.span;
    }

    /// Lê explicitamente os dados de expectativa sintática para diagnóstico
    /// interno.
    fn touch_expected_token(expected: &ExpectedToken) {
        match expected {
            ExpectedToken::Ident | ExpectedToken::Number => {}
            ExpectedToken::Keyword(sym) => {
                let _ = *sym;
            }
            ExpectedToken::Exact(kind) => {
                let _ = *kind;
            }
        }
    }
}
