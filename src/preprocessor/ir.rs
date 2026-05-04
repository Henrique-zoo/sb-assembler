//! Intermediate Representation (IR) do pré-processador.
//!
//! Este módulo define as estruturas de dados que representam diretivas e
//! construções reconhecidas pelo parser, em um formato mais estável para o
//! estágio de execução.
//!
//! Papel no pipeline:
//! 1. `detection` identifica uma tentativa de diretiva (heurístico/permissivo);
//! 2. `parser` valida sintaxe e converte para IR;
//! 3. `execute` aplica semântica a partir da IR.
//!
//! Motivação:
//! - evita acoplamento direto entre parser e estado mutável do pré-processador;
//! - melhora legibilidade e testabilidade (dados explícitos em vez de slices
//!   de token soltos);
//! - deixa o parser estritamente sintático, sem efeitos colaterais.
//!
//! Escopo:
//! - Este módulo não executa nada e não decide semântica;
//! - Ele apenas modela a informação extraída de cada linha/diretiva.

use crate::{
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::types::Section,
};

/// Alias para parâmetro formal de macro.
///
/// Mantido como alias para permitir evolução futura da representação sem
/// alterar assinaturas de alto nível.
pub(crate) type Param = Symbol;

/// Cabecalho de definição de macro.
///
/// Exemplo: `ROT: MACRO &A, &B`
/// - `name` = `ROT`
/// - `params` = [`&A`, `&B`] (internados como simbolos)
/// - `span` = faixa da diretiva inteira no fonte
#[derive(Debug, Clone)]
pub(crate) struct MacroHeader {
    pub name: Symbol,
    pub params: Vec<Param>,
    pub span: Span,
}

/// Definição completa de macro armazenada na tabela de macros.
///
/// O corpo e mantido como linhas de tokens para permitir expansão posterior.
#[derive(Debug, Clone)]
pub(crate) struct Macro {
    pub header: MacroHeader,
    pub body: Vec<Vec<Token>>,
}

/// Operando simples aceito em diretivas do pré-processador.
///
/// Usado em `EQU` e `IF`, por exemplo.
#[derive(Debug, Clone)]
pub(crate) enum Operand {
    Number(Symbol),
    Ident(Symbol),
}

/// Declaração de seção parseada (`SECTION TEXT` ou `SECTION DATA`).
#[derive(Debug, Clone)]
pub(crate) struct SectionDecl {
    /// Seção alvo.
    pub section: Section,
    /// Span principal para diagnósticos (diretiva inteira consumida).
    pub span: Span,
}

/// Declaração `EQU` parseada.
#[derive(Debug, Clone)]
pub(crate) struct EquDecl {
    /// Alias definido pela diretiva.
    pub alias: Symbol,
    /// Valor associado ao alias.
    pub value: Operand,
    /// Span principal para diagnósticos (diretiva inteira consumida).
    pub span: Span,
}

/// Declaração `IF` parseada.
#[derive(Debug, Clone)]
pub(crate) struct IfDecl {
    /// Condicao usada para decidir inclusão/skip da próximo linha.
    pub cond: Operand,
    /// Span principal para diagnósticos (diretiva inteira consumida).
    pub span: Span,
}

/// Chamada de macro parseada.
#[derive(Debug, Clone)]
pub(crate) struct MacroCall {
    /// Nome da macro chamada.
    pub name: Symbol,
    /// Argumentos posicionais.
    pub args: Vec<Symbol>,
    /// Span principal para diagnósticos (chamada inteira consumida).
    pub span: Span,
}

/// Linha do corpo de macro em representação estruturada.
///
/// Cada item pode ser literal (token bruto) ou referência a parâmetro formal.
pub(crate) struct MacroBodyLine {
    pub items: Vec<MacroBodyItem>,
    pub span: Span,
}

/// Unidade semântica de uma linha do corpo de macro.
pub(crate) enum MacroBodyItem {
    /// Token literal preservado como apareceu no body.
    Literal(Token),
    /// Referência a parâmetro formal (ex.: `&ARG`) e span local.
    ParamRef(Symbol, Span),
}
