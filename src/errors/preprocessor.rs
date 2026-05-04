//! Erros produzidos no estágio de pré-processamento.
//!
//! Este módulo modela diagnósticos para diretivas e expansão de macros,
//! separando:
//! - erros sintáticos de cabeçalho/parâmetros de macro;
//! - erros sintáticos genéricos compartilhados entre diretivas;
//! - erros semânticos de diretivas e uso de macros;
//! - o envelope final com `Span` ([`PreprocessorError`]).

use crate::{interner::Symbol, lexer::Span};

/// Subcategoria de erro para parâmetros formais em cabeçalho de macro.
#[derive(Debug, Clone)]
pub(crate) enum InvalidParamKind {
    /// Token de identificador inválido após `&`.
    InvalidParamIdent,
    /// Ausência de `&` ao iniciar parâmetro formal.
    NoAmpersand,
    /// Vírgula sem parâmetro seguinte (ex.: lista terminando com `,`).
    UnexpectedComma,
}

/// Subcategoria de erro para argumentos em uma chamada de macro
#[derive(Debug, Clone)]
pub(crate) enum InvalidArgKind {
    /// Token de identificador inválido
    InvalidArgIdent,
    /// Vírgula sem argumento seguinte
    UnexpectedComma,
}

/// Erros sintáticos específicos de cabeçalho `MACRO`.
#[derive(Debug, Clone)]
pub(crate) enum MacroHeaderErrorKind {
    /// `:` obrigatório ausente após o rótulo.
    MissingColon,
    /// Label inicial inválida.
    InvalidLabel,
    /// Erro dentro da lista de parâmetros formais.
    InvalidParam(InvalidParamKind),
}

#[derive(Debug, Clone)]
pub(crate) enum MacroCallErrorKind {
    /// Chamada de Macro não definida.
    UndefinedMacro,
    /// Chamada de Macro com menos argumentos do que o necessário.
    MissingArguments,
    /// Erro dentro da lista de argumentos.
    InvalidArg(InvalidArgKind),
}

/// Erros sintáticos específicos da diretiva `IF`
#[derive(Debug, Clone)]
pub(crate) enum IfDirectiveErrorKind {
    /// Condição ausente após palavra-chave `IF`
    MissingCondition,
    /// Token de condição de tipo inválido
    InvalidConditionType,
}

/// Erros sintáticos específicos da diretiva `EQU`
#[derive(Debug, Clone)]
pub(crate) enum EquDirectiveErrorKind {
    /// Token do valor que o `EQU` atribui de tipo inválido
    InvalidValueType,
}

/// Erros sintáticos específicos de declaração de seções
#[derive(Debug, Clone)]
pub(crate) enum SectionDeclarationErrorKind {
    /// Keyword `Section` obrigatória ausente
    MissingSectionKeyword,
}

/// Diretiva alvo associada a um erro sintático genérico.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DirectiveKind {
    MacroHeader,
    EndMacro,
    Equ,
    If,
    Section,
    MacroCall,
}

/// Erros sintáticos compartilhados entre diferentes diretivas.
#[derive(Debug, Clone)]
pub(crate) enum DirectiveSyntaxErrorKind {
    /// Tokens extras para a gramática daquela diretiva.
    TrailingTokens { directive: DirectiveKind },
    /// Token faltando para a gramática daquela diretiva (na prática, é unreachable)
    MissingToken {
        directive: DirectiveKind,
        token_missed: Option<Symbol>,
    },
    /// Declaração inválida - Erro de fallback para tratar casos supostamente unreacheable
    InvalidDeclaration { directive: DirectiveKind },
    /// Token inesperado para a gramática daquela diretiva
    UnexpectedToken {
        directive: DirectiveKind,
        expected_token: Symbol,
    },
    /// Token inesperado específico para a gramática daquela diretiva - trato erros de sintaxe "clássicos" para melhorar a mensagem de erro
    ForbiddenToken { directive: DirectiveKind },
}

/// Categorias de erro do pré-processador.
///
/// Inclui falhas de definição/expansão de macro e uso inválido de diretivas.
#[derive(Debug, Clone)]
pub(crate) enum PreprocessorErrorKind {
    /// `ENDMACRO` apareceu fora de contexto de definição.
    UnexpectedEndMacro,
    /// Arquivo terminou com macro ainda aberta.
    UnterminatedMacro,
    /// Nova definição para macro já existente.
    MacroAlreadyDefined(Symbol),
    /// Quantidade de argumentos em chamada diverge da definição.
    WrongArgCount { expected: usize, found: usize },
    /// Cabeçalho de macro malformado.
    InvalidMacroHeader(MacroHeaderErrorKind),
    /// Chamada para macro inválida
    InvalidMacroCall(MacroCallErrorKind),
    /// Diretiva IF inválida
    InvalidIfDirective(IfDirectiveErrorKind),
    /// Diretiva EQU inválida
    InvalidEquDirective(EquDirectiveErrorKind),
    /// Declaração de Seção inválida
    InvalidSectionDeclaration(SectionDeclarationErrorKind),
    /// Erro sintático genérico de diretiva.
    InvalidDirectiveSyntax(DirectiveSyntaxErrorKind),
    /// Diretiva inválida ou malformada.
    InvalidDirective,
}

/// Diagnóstico final emitido pelo pré-processador.
///
/// Combina categoria do erro com localização no código-fonte.
#[derive(Debug, Clone)]
pub(crate) struct PreprocessorError {
    /// Categoria específica da falha.
    pub kind: PreprocessorErrorKind,
    /// Região do fonte associada ao problema.
    pub span: Span,
}
