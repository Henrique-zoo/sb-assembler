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
use std::{error::Error, fmt};

use crate::{
    assembly::one_pass::OnePassAssembler,
    errors::{
        AssemblyError, AssemblyErrorKind, DirectiveKind, DirectiveSyntaxErrorKind,
        EquDirectiveSemanticErrorKind, EquDirectiveSyntaticErrorKind, ExpectedToken,
        IfDirectiveSemanticErrorKind, IfDirectiveSyntaticErrorKind, InvalidArgKind,
        InvalidParamKind, LexerError, LexerErrorKind, MacroCallSemanticErrorKind,
        MacroCallSyntaticErrorKind, MacroHeaderErrorKind, NumberContext, PreprocessorError,
        PreprocessorErrorKind,
    },
    file_creator::FileCreator,
    interner::{Interner, Symbol},
    language::{LanguageSymbols, instructions::InstructionSet},
    lexer::{Lexer, Span, Token, TokenKind},
    parser::{
        Parser,
        types::{NodeSpans, ParsedProgram},
    },
    preprocessor::{LogicalLine, PreprocessedProgram, Preprocessor},
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

/// Estágio do pipeline que produziu um diagnóstico do assembler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblerStage {
    /// Análise léxica.
    Lexer,
    /// Pré-processamento.
    Preprocessor,
    /// Parsing do programa pré-processado.
    Parser,
    /// Montagem e resolução de símbolos.
    Assembly,
}

impl AssemblerStage {
    fn label(self) -> &'static str {
        match self {
            Self::Lexer => "Erro léxico",
            Self::Preprocessor => "Erro de pré-processamento",
            Self::Parser => "Erro de parsing",
            Self::Assembly => "Erro de montagem",
        }
    }
}

/// Diagnóstico emitido pelo pipeline do assembler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerDiagnostic {
    stage: AssemblerStage,
    span: Option<Span>,
    line: Option<u32>,
    column: Option<u32>,
    message: String,
}

impl AssemblerDiagnostic {
    fn new(stage: AssemblerStage, span: Span, message: String) -> Self {
        let span = (span != Span::default()).then_some(span);
        let line = span.map(|span| span.line);
        let column = span.map(|span| span.column);

        Self {
            stage,
            span,
            line,
            column,
            message,
        }
    }

    /// Retorna o estágio que produziu o diagnóstico.
    pub fn stage(&self) -> AssemblerStage {
        self.stage
    }

    /// Retorna a linha do diagnóstico, quando conhecida.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Retorna a coluna do diagnóstico, quando conhecida.
    pub fn column(&self) -> Option<u32> {
        self.column
    }

    /// Retorna a mensagem textual do diagnóstico.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AssemblerDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                write!(
                    f,
                    "{} em {line}:{column}: {}",
                    self.stage.label(),
                    self.message
                )
            }
            _ => write!(f, "{}: {}", self.stage.label(), self.message),
        }
    }
}

/// Erro retornado pela fachada de alto nível do assembler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerError {
    source: String,
    diagnostics: Vec<AssemblerDiagnostic>,
}

impl AssemblerError {
    fn new(source: &str, diagnostics: Vec<AssemblerDiagnostic>) -> Self {
        Self {
            source: source.to_owned(),
            diagnostics,
        }
    }

    /// Retorna os diagnósticos acumulados pelo estágio que falhou.
    pub fn diagnostics(&self) -> &[AssemblerDiagnostic] {
        &self.diagnostics
    }

    fn source_line(&self, line_number: u32) -> Option<&str> {
        self.source
            .lines()
            .nth(line_number.saturating_sub(1) as usize)
    }

    fn render_snippet(
        &self,
        f: &mut fmt::Formatter<'_>,
        diagnostic: &AssemblerDiagnostic,
    ) -> fmt::Result {
        let Some(span) = diagnostic.span else {
            return Ok(());
        };
        let Some(line_text) = self.source_line(span.line) else {
            return Ok(());
        };

        let line_number = span.line.to_string();
        let gutter_width = line_number.len();
        let marker_prefix = Self::marker_prefix(line_text, span.column);
        let marker_width = Self::marker_width(&self.source, span);

        writeln!(f, "{:>width$} |", "", width = gutter_width)?;
        writeln!(
            f,
            "{line_number:>width$} | {line_text}",
            width = gutter_width
        )?;
        writeln!(
            f,
            "{:>width$} | {marker_prefix}{} {}",
            "",
            "^".repeat(marker_width),
            diagnostic.message,
            width = gutter_width
        )
    }

    fn marker_prefix(line_text: &str, column: u32) -> String {
        line_text
            .chars()
            .take(column.saturating_sub(1) as usize)
            .map(|ch| if ch == '\t' { '\t' } else { ' ' })
            .collect()
    }

    fn marker_width(source: &str, span: Span) -> usize {
        source
            .get(span.pos..span.pos.saturating_add(span.len))
            .map(|slice| slice.chars().count().max(1))
            .unwrap_or_else(|| span.len.max(1))
    }
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, diagnostic) in self.diagnostics.iter().enumerate() {
            writeln!(f, "{diagnostic}")?;
            self.render_snippet(f, diagnostic)?;

            if idx + 1 < self.diagnostics.len() {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

impl Error for AssemblerError {}

struct ParserDiagnostic {
    section: &'static str,
    message: String,
    span: Span,
}

enum PipelineError {
    Lexer(LexerError),
    Preprocessor(Vec<PreprocessorError>),
    Parser(Vec<ParserDiagnostic>),
    Assembly(Vec<AssemblyError>),
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

    /// Executa léxico e pré-processamento.
    ///
    /// Retorna erro quando algum diagnóstico é emitido.
    pub fn process(mut self) -> Result<(), AssemblerError> {
        match self.run_preprocessor() {
            Ok(_) => Ok(()),
            Err(err) => Err(self.assembler_error(err)),
        }
    }

    /// Gera os arquivos `.obj` e `.pen` a partir de um fonte preprocessado.
    ///
    /// O método interrompe o pipeline no primeiro estágio com erro e não grava
    /// artefatos parciais.
    pub fn generate_obj_and_pen_files(mut self, file_name: &str) -> Result<(), AssemblerError> {
        let preprocessed_tokens = match self.run_preprocessor() {
            Ok(preprocessed_tokens) => preprocessed_tokens,
            Err(err) => return Err(self.assembler_error(err)),
        };

        let parsed_program = match self.parse_preprocessed_program(preprocessed_tokens) {
            Ok(parsed_program) => parsed_program,
            Err(errors) => {
                return Err(self.assembler_error(PipelineError::Parser(errors)));
            }
        };

        let isa = InstructionSet::new();
        let one_pass_assembler = OnePassAssembler::new(&self.interner, &isa);
        let final_program = match one_pass_assembler.assemble(parsed_program) {
            Ok(final_program) => final_program,
            Err(errors) => {
                return Err(self.assembler_error(PipelineError::Assembly(errors)));
            }
        };

        FileCreator::create_pending_output_file(
            &format!("{}.pen", file_name),
            &final_program.pending.words,
        );
        FileCreator::create_object_output_file(
            &format!("{}.obj", file_name),
            &final_program.object.words,
        );

        Ok(())
    }

    /// Gera o arquivo `.pre` a partir de um fonte assembly.
    ///
    /// O arquivo só é escrito quando o léxico e o pré-processamento terminam
    /// sem diagnósticos.
    pub fn generate_preprocessed_file(mut self, file_name: &str) -> Result<(), AssemblerError> {
        let preprocessed_tokens = match self.run_preprocessor() {
            Ok(preprocessed_tokens) => preprocessed_tokens,
            Err(err) => return Err(self.assembler_error(err)),
        };

        FileCreator::create_preprocessed_output_file(
            file_name,
            &preprocessed_tokens,
            &self.interner,
        );

        Ok(())
    }

    fn run_preprocessor(&mut self) -> Result<PreprocessedProgram, PipelineError> {
        let tokens = self.collect_tokens().map_err(PipelineError::Lexer)?;
        let mut preprocessor = Preprocessor::new(&self.language_symbols);

        preprocessor
            .process(tokens, &mut self.interner)
            .map_err(PipelineError::Preprocessor)
    }

    fn collect_tokens(&mut self) -> Result<Vec<Token>, LexerError> {
        let normalized_source = self.source.to_ascii_uppercase();
        let lexer = Lexer::new(normalized_source.as_str(), &mut self.interner);

        lexer.collect::<Result<Vec<_>, _>>()
    }

    fn parse_preprocessed_program(
        &self,
        program: PreprocessedProgram,
    ) -> Result<ParsedProgram, Vec<ParserDiagnostic>> {
        let parser = Parser::new(&self.language_symbols);
        let mut node_spans = NodeSpans::new();
        let mut diagnostics = Vec::new();
        let mut text = Vec::new();
        let mut data = Vec::new();

        for line in &program.text {
            match parser.parse_text_line(&line.content, &mut node_spans) {
                Ok(parsed_line) => text.push(parsed_line),
                Err(message) => diagnostics.push(ParserDiagnostic {
                    section: "TEXT",
                    message: message.to_owned(),
                    span: Self::logical_line_span(line),
                }),
            }
        }

        for line in &program.data {
            match parser.parse_data_line(&line.content, &mut node_spans) {
                Ok(parsed_line) => data.push(parsed_line),
                Err(message) => diagnostics.push(ParserDiagnostic {
                    section: "DATA",
                    message: message.to_owned(),
                    span: Self::logical_line_span(line),
                }),
            }
        }

        if diagnostics.is_empty() {
            Ok(ParsedProgram {
                text,
                data,
                node_spans,
            })
        } else {
            Err(diagnostics)
        }
    }

    fn logical_line_span(line: &LogicalLine) -> Span {
        match (line.content.first(), line.content.last()) {
            (Some(first), Some(last)) => {
                let end = last.span.pos.saturating_add(last.span.len);

                Span {
                    pos: first.span.pos,
                    line: first.span.line,
                    column: first.span.column,
                    len: end.saturating_sub(first.span.pos),
                }
            }
            _ => line.terminator.span,
        }
    }

    fn assembler_error(&self, err: PipelineError) -> AssemblerError {
        let diagnostics = match err {
            PipelineError::Lexer(err) => vec![self.lexer_diagnostic(&err)],
            PipelineError::Preprocessor(errors) => self.preprocessor_diagnostics(&errors),
            PipelineError::Parser(errors) => self.parser_diagnostics(&errors),
            PipelineError::Assembly(errors) => self.assembly_diagnostics(&errors),
        };

        AssemblerError::new(self.source, diagnostics)
    }

    fn lexer_diagnostic(&self, err: &LexerError) -> AssemblerDiagnostic {
        let message = match &err.kind {
            LexerErrorKind::InvalidChar(ch) => {
                format!("caractere inválido '{}'", ch.escape_default())
            }
            LexerErrorKind::InvalidNumber(text) => {
                format!("número inválido `{text}`")
            }
        };

        AssemblerDiagnostic::new(AssemblerStage::Lexer, err.span, message)
    }

    fn preprocessor_diagnostics(&self, errors: &[PreprocessorError]) -> Vec<AssemblerDiagnostic> {
        errors
            .iter()
            .map(|err| {
                AssemblerDiagnostic::new(
                    AssemblerStage::Preprocessor,
                    err.span,
                    self.format_preprocessor_error(&err.kind),
                )
            })
            .collect()
    }

    fn parser_diagnostics(&self, errors: &[ParserDiagnostic]) -> Vec<AssemblerDiagnostic> {
        errors
            .iter()
            .map(|err| {
                AssemblerDiagnostic::new(
                    AssemblerStage::Parser,
                    err.span,
                    format!("seção {}: {}", err.section, err.message),
                )
            })
            .collect()
    }

    fn assembly_diagnostics(&self, errors: &[AssemblyError]) -> Vec<AssemblerDiagnostic> {
        errors
            .iter()
            .map(|err| {
                AssemblerDiagnostic::new(
                    AssemblerStage::Assembly,
                    err.span,
                    self.format_assembly_error(&err.kind),
                )
            })
            .collect()
    }

    fn format_preprocessor_error(&self, kind: &PreprocessorErrorKind) -> String {
        match kind {
            PreprocessorErrorKind::UnknownLineSyntax => {
                "linha fora de seção sem diretiva reconhecida".to_owned()
            }
            PreprocessorErrorKind::UnexpectedEndMacro => {
                "`ENDMACRO` encontrado fora de uma definição de macro".to_owned()
            }
            PreprocessorErrorKind::UnterminatedMacro => {
                "definição de macro sem `ENDMACRO`".to_owned()
            }
            PreprocessorErrorKind::MacroAlreadyDefined(sym) => {
                format!("macro `{}` já foi definida", self.symbol_name(*sym))
            }
            PreprocessorErrorKind::InvalidMacroHeader(kind) => match kind {
                MacroHeaderErrorKind::InvalidLabel => {
                    "cabeçalho de macro precisa começar com rótulo".to_owned()
                }
                MacroHeaderErrorKind::InvalidParam(kind) => {
                    format!(
                        "parâmetro inválido em macro: {}",
                        Self::invalid_param_message(kind)
                    )
                }
            },
            PreprocessorErrorKind::InvalidMacroCallSyntatic(kind) => match kind {
                MacroCallSyntaticErrorKind::InvalidArg(kind) => {
                    format!(
                        "argumento inválido em chamada de macro: {}",
                        Self::invalid_arg_message(kind)
                    )
                }
            },
            PreprocessorErrorKind::InvalidMacroCallSemantic(kind) => match kind {
                MacroCallSemanticErrorKind::UndefinedMacro => "macro não definida".to_owned(),
                MacroCallSemanticErrorKind::MissingArguments => {
                    "chamada de macro com argumentos ausentes".to_owned()
                }
                MacroCallSemanticErrorKind::WrongArgCount { expected, found } => {
                    format!(
                        "quantidade de argumentos inválida: esperado {expected}, encontrado {found}"
                    )
                }
            },
            PreprocessorErrorKind::InvalidIfDirectiveSyntatic(kind) => match kind {
                IfDirectiveSyntaticErrorKind::MissingCondition => {
                    "diretiva `IF` sem condição".to_owned()
                }
                IfDirectiveSyntaticErrorKind::InvalidConditionType => {
                    "condição de `IF` deve ser número ou identificador".to_owned()
                }
            },
            PreprocessorErrorKind::InvalidIfDirectiveSemantic(kind) => match kind {
                IfDirectiveSemanticErrorKind::ConditionNumberOverflow { value } => {
                    format!(
                        "condição de `IF` fora do intervalo: `{}`",
                        self.symbol_name(*value)
                    )
                }
                IfDirectiveSemanticErrorKind::InvalidConditionNumber { value } => {
                    format!(
                        "condição numérica inválida em `IF`: `{}`",
                        self.symbol_name(*value)
                    )
                }
                IfDirectiveSemanticErrorKind::UndefinedIdentifier { ident } => {
                    format!(
                        "identificador indefinido em `IF`: {}",
                        self.token_kind_name(*ident)
                    )
                }
                IfDirectiveSemanticErrorKind::MissingNextLine => {
                    "`IF` precisa controlar uma linha seguinte".to_owned()
                }
            },
            PreprocessorErrorKind::InvalidEquDirectiveSyntatic(kind) => match kind {
                EquDirectiveSyntaticErrorKind::InvalidValueType => {
                    "valor de `EQU` deve ser número ou identificador".to_owned()
                }
            },
            PreprocessorErrorKind::InvalidEquDirectiveSemantic(kind) => match kind {
                EquDirectiveSemanticErrorKind::UndefinedSymbol { symbol } => {
                    format!(
                        "símbolo `{}` usado em `EQU` não foi definido",
                        self.symbol_name(*symbol)
                    )
                }
                EquDirectiveSemanticErrorKind::InvalidValueNumber { value } => {
                    format!(
                        "valor numérico inválido em `EQU`: `{}`",
                        self.symbol_name(*value)
                    )
                }
                EquDirectiveSemanticErrorKind::ValueNumberOverflow { value } => {
                    format!(
                        "valor de `EQU` fora do intervalo: `{}`",
                        self.symbol_name(*value)
                    )
                }
            },
            PreprocessorErrorKind::InvalidDirectiveSyntax(kind) => {
                self.format_directive_syntax_error(kind)
            }
        }
    }

    fn format_directive_syntax_error(&self, kind: &DirectiveSyntaxErrorKind) -> String {
        match kind {
            DirectiveSyntaxErrorKind::TrailingTokens { directive } => {
                format!(
                    "tokens extras na diretiva `{}`",
                    Self::directive_name(*directive)
                )
            }
            DirectiveSyntaxErrorKind::MissingToken {
                directive,
                expected,
            } => {
                format!(
                    "token ausente na diretiva `{}`: esperado {}",
                    Self::directive_name(*directive),
                    self.expected_token_name(*expected)
                )
            }
            DirectiveSyntaxErrorKind::InvalidDeclaration { directive } => {
                format!(
                    "declaração inválida da diretiva `{}`",
                    Self::directive_name(*directive)
                )
            }
            DirectiveSyntaxErrorKind::UnexpectedToken {
                directive,
                expected,
            } => {
                format!(
                    "token inesperado na diretiva `{}`: esperado {}",
                    Self::directive_name(*directive),
                    self.expected_token_name(*expected)
                )
            }
            DirectiveSyntaxErrorKind::ForbiddenToken { directive } => {
                format!(
                    "token proibido na diretiva `{}`",
                    Self::directive_name(*directive)
                )
            }
        }
    }

    fn format_assembly_error(&self, kind: &AssemblyErrorKind) -> String {
        match kind {
            AssemblyErrorKind::NoRotuleInDataLine => {
                "linha da seção DATA precisa ter rótulo".to_owned()
            }
            AssemblyErrorKind::SymbolAlreadyDefined {
                symbol,
                first_defined_at,
            } => format!(
                "símbolo `{}` já definido anteriormente em {}:{}",
                self.symbol_name(*symbol),
                first_defined_at.line,
                first_defined_at.column
            ),
            AssemblyErrorKind::UndefinedSymbol { symbol } => {
                format!("símbolo `{}` não foi definido", self.symbol_name(*symbol))
            }
            AssemblyErrorKind::InvalidNumber { value, context } => {
                format!(
                    "número inválido `{}` em {}",
                    self.symbol_name(*value),
                    Self::number_context_name(*context)
                )
            }
            AssemblyErrorKind::NumberOverflow { value, context } => {
                format!(
                    "número `{}` fora do intervalo em {}",
                    self.symbol_name(*value),
                    Self::number_context_name(*context)
                )
            }
            AssemblyErrorKind::InvalidSpaceAmount { value } => {
                format!(
                    "quantidade inválida em `SPACE`: `{}`",
                    self.symbol_name(*value)
                )
            }
            AssemblyErrorKind::AddressOverflow { address } => {
                format!("endereço calculado não cabe na palavra da máquina: {address}")
            }
            AssemblyErrorKind::InvalidMnemonic { mnemonic } => {
                format!("mnemônico inválido na tabela de instruções: {mnemonic:?}")
            }
        }
    }

    fn invalid_param_message(kind: &InvalidParamKind) -> &'static str {
        match kind {
            InvalidParamKind::InvalidParamIdent => "esperado identificador após `&`",
            InvalidParamKind::NoAmpersand => "parâmetro deve começar com `&`",
            InvalidParamKind::UnexpectedComma => "vírgula sem parâmetro seguinte",
        }
    }

    fn invalid_arg_message(kind: &InvalidArgKind) -> &'static str {
        match kind {
            InvalidArgKind::InvalidArgIdent => "esperado identificador ou número",
            InvalidArgKind::UnexpectedComma => "vírgula sem argumento seguinte",
        }
    }

    fn expected_token_name(&self, expected: ExpectedToken) -> String {
        match expected {
            ExpectedToken::Ident => "identificador".to_owned(),
            ExpectedToken::Number => "número".to_owned(),
            ExpectedToken::Keyword(sym) => format!("keyword `{}`", self.symbol_name(sym)),
            ExpectedToken::Exact(kind) => self.token_kind_name(kind),
        }
    }

    fn token_kind_name(&self, kind: TokenKind) -> String {
        match kind {
            TokenKind::Ident(sym) => format!("identificador `{}`", self.symbol_name(sym)),
            TokenKind::Number(sym) => format!("número `{}`", self.symbol_name(sym)),
            TokenKind::Ampersand => "`&`".to_owned(),
            TokenKind::Comma => "`,`".to_owned(),
            TokenKind::Colon => "`:`".to_owned(),
            TokenKind::Plus => "`+`".to_owned(),
            TokenKind::Minus => "`-`".to_owned(),
            TokenKind::NewLine => "quebra de linha".to_owned(),
        }
    }

    fn directive_name(directive: DirectiveKind) -> &'static str {
        match directive {
            DirectiveKind::MacroHeader => "MACRO",
            DirectiveKind::MacroBody => "corpo de MACRO",
            DirectiveKind::EndMacro => "ENDMACRO",
            DirectiveKind::Equ => "EQU",
            DirectiveKind::If => "IF",
            DirectiveKind::Section => "SECTION",
            DirectiveKind::MacroCall => "chamada de macro",
        }
    }

    fn number_context_name(context: NumberContext) -> &'static str {
        match context {
            NumberContext::ConstValue => "valor de `CONST`",
            NumberContext::SpaceAmount => "quantidade de `SPACE`",
            NumberContext::AddressOffset => "deslocamento de endereço",
        }
    }

    fn symbol_name(&self, symbol: Symbol) -> String {
        self.interner
            .get_str(symbol)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("<sym:{symbol}>"))
    }
}
