//! Erros produzidos no estágio de pré-processamento.
//!
//! Este módulo modela diagnósticos para diretivas e expansão de macros,
//! separando:
//! - erros sintáticos de cabeçalho/parâmetros de macro;
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

/// Erros sintáticos específicos de cabeçalho `MACRO`.
#[derive(Debug, Clone)]
pub(crate) enum MacroHeaderErrorKind {
    /// `:` obrigatório ausente após o rótulo.
    MissingColon,
    /// Tokens extras/inesperados no cabeçalho.
    TrailingTokens,
    /// Label inicial inválida.
    InvalidLabel,
    /// Erro dentro da lista de parâmetros formais.
    InvalidParam(InvalidParamKind),
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
    /// Chamada para macro inexistente.
    MacroNotDefined(Symbol),
    /// Quantidade de argumentos em chamada diverge da definição.
    WrongArgCount { expected: usize, found: usize },
    /// Cabeçalho de macro malformado.
    InvalidMacroHeader(MacroHeaderErrorKind),
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
