//! Erros produzidos no estágio de pré-processamento.
//!
//! Este módulo modela diagnósticos para diretivas e expansão de macros,
//! separando:
//! - erros sintáticos de cabeçalho/parâmetros de macro;
//! - erros sintáticos genéricos compartilhados entre diretivas;
//! - erros semânticos de diretivas e uso de macros;
//! - o envelope final com `Span` ([`PreprocessorError`]).

use crate::{
    interner::Symbol,
    lexer::{Span, TokenKind},
};

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
    /// Label inicial inválida.
    InvalidLabel,
    /// Erro dentro da lista de parâmetros formais.
    InvalidParam(InvalidParamKind),
}

#[derive(Debug, Clone)]
pub(crate) enum MacroCallSyntaticErrorKind {
    /// Erro dentro da lista de argumentos.
    InvalidArg(InvalidArgKind),
}

#[derive(Debug, Clone)]
pub(crate) enum MacroCallSemanticErrorKind {
    /// Chamada de Macro não definida.
    UndefinedMacro,
    /// Chamada de Macro com menos argumentos do que o necessário.
    MissingArguments,
    /// Quantidade de argumentos em chamada diverge da definição.
    WrongArgCount { expected: usize, found: usize },
}

/// Erros sintáticos específicos da diretiva `IF`
#[derive(Debug, Clone)]
pub(crate) enum IfDirectiveSyntaticErrorKind {
    /// Condição ausente após palavra-chave `IF`
    MissingCondition,
    /// Token de condição de tipo inválido
    InvalidConditionType,
}

/// Erros semânticos específicos da diretiva `IF`.
#[derive(Debug, Clone)]
pub(crate) enum IfDirectiveSemanticErrorKind {
    /// Número associado à condição é grande demais para a arquitetura de 16 bits.
    ConditionNumberOverflow { value: Symbol },
    /// Literal numérico da condição não pôde ser parseado.
    InvalidConditionNumber { value: Symbol },
    /// Identificador da condição não foi definido.
    UndefinedIdentifier { ident: TokenKind },
    /// Linha seguinte não existe
    MissingNextLine,
}

/// Erros sintáticos específicos da diretiva `EQU`
#[derive(Debug, Clone)]
pub(crate) enum EquDirectiveSyntaticErrorKind {
    /// Token do valor que o `EQU` atribui de tipo inválido
    InvalidValueType,
}

/// Erros semânticos específicos da diretiva `EQU`.
#[derive(Debug, Clone)]
pub(crate) enum EquDirectiveSemanticErrorKind {
    /// Alias `EQU` usado antes de ser definido.
    UndefinedSymbol { symbol: Symbol },
    /// Literal numérico do valor não pôde ser parseado.
    InvalidValueNumber { value: Symbol },
    /// Número associado ao valor estoura o intervalo de 16 bits.
    ValueNumberOverflow { value: Symbol },
}

/// Diretiva alvo associada a um erro sintático genérico.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DirectiveKind {
    MacroHeader,
    MacroBody,
    EndMacro,
    Equ,
    If,
    Section,
    MacroCall,
}

/// Token ou classe de token esperada por uma gramática de diretiva.
///
/// Diferente de [`TokenKind`], este tipo permite representar expectativas
/// genéricas como "um identificador" sem fabricar um `Symbol` artificial para
/// diagnóstico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedToken {
    /// Qualquer identificador.
    Ident,
    /// Qualquer literal numérico.
    Number,
    /// Keyword específica internada.
    Keyword(Symbol),
    /// Token fixo específico (`:`, `,`, `&`, `+`, `-` etc.).
    Exact(TokenKind),
}

impl ExpectedToken {
    /// Converte um [`TokenKind`] concreto na expectativa equivalente.
    ///
    /// `Ident(_)` e `Number(_)` viram expectativas por classe; os demais tokens
    /// são preservados como tokens exatos.
    pub(crate) fn from_token_kind(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Ident(_) => Self::Ident,
            TokenKind::Number(_) => Self::Number,
            kind => Self::Exact(kind),
        }
    }
}

/// Erros sintáticos compartilhados entre diferentes diretivas.
#[derive(Debug, Clone)]
pub(crate) enum DirectiveSyntaxErrorKind {
    /// Tokens extras para a gramática daquela diretiva.
    TrailingTokens { directive: DirectiveKind },
    /// Token faltando para a gramática daquela diretiva (na prática, é unreachable)
    MissingToken {
        directive: DirectiveKind,
        expected: ExpectedToken,
    },
    /// Declaração inválida - Erro de fallback para tratar casos supostamente unreacheable
    InvalidDeclaration { directive: DirectiveKind },
    /// Token inesperado para a gramática daquela diretiva
    UnexpectedToken {
        directive: DirectiveKind,
        expected: ExpectedToken,
    },
    /// Token inesperado específico para a gramática daquela diretiva - trato erros de sintaxe "clássicos" para melhorar a mensagem de erro
    ForbiddenToken { directive: DirectiveKind },
}

/// Categorias de erro do pré-processador.
///
/// Inclui falhas de definição/expansão de macro e uso inválido de diretivas.
#[derive(Debug, Clone)]
pub(crate) enum PreprocessorErrorKind {
    /// Linha na seção None não é  de nenhum tipo conhecido
    UnknownLineSyntax,
    /// `ENDMACRO` apareceu fora de contexto de definição.
    UnexpectedEndMacro,
    /// Arquivo terminou com macro ainda aberta.
    UnterminatedMacro,
    /// Nova definição para macro já existente.
    MacroAlreadyDefined(Symbol),
    /// Cabeçalho de macro malformado.
    InvalidMacroHeader(MacroHeaderErrorKind),
    /// Chamada de macro com erro sintático.
    InvalidMacroCallSyntatic(MacroCallSyntaticErrorKind),
    /// Chamada de macro com erro semântico.
    InvalidMacroCallSemantic(MacroCallSemanticErrorKind),
    /// Diretiva IF com erro sintático.
    InvalidIfDirectiveSyntatic(IfDirectiveSyntaticErrorKind),
    /// Diretiva IF com erro semântico.
    InvalidIfDirectiveSemantic(IfDirectiveSemanticErrorKind),
    /// Diretiva EQU com erro sintático.
    InvalidEquDirectiveSyntatic(EquDirectiveSyntaticErrorKind),
    /// Diretiva EQU com erro semântico.
    InvalidEquDirectiveSemantic(EquDirectiveSemanticErrorKind),
    /// Erro sintático genérico de diretiva.
    InvalidDirectiveSyntax(DirectiveSyntaxErrorKind),
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
