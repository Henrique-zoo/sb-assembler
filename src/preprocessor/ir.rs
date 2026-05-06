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
    lexer::{Span, Token, TokenKind},
    preprocessor::types::Section,
};

/// Identificador estável de um nó da IR.
///
/// O `NodeId` permite desacoplar metadados de diagnóstico (`Span`) da
/// representação principal da IR. O mapeamento `NodeId -> Span` fica na
/// tabela lateral mantida pelo `Preprocessor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(pub u32);

/// Alias para parâmetro formal de macro.
///
/// Mantido como alias para permitir evolução futura da representação sem
/// alterar assinaturas de alto nível.
pub(crate) type Param = Symbol;

/// Cabeçalho de definição de macro.
///
/// Exemplo: `ROT: MACRO &A, &B`
/// - `name` = `ROT`
/// - `params` = [`&A`, `&B`] (internados como símbolos)
/// - `node_id` aponta para o span da diretiva inteira no fonte
#[derive(Debug, Clone)]
pub(crate) struct MacroHeader {
    pub node_id: NodeId,
    pub name: Symbol,
    pub params: Vec<Param>,
}

/// Definição completa de macro armazenada na tabela de macros.
///
/// O corpo é mantido como linhas de tokens para permitir expansão posterior.
#[derive(Debug, Clone)]
pub(crate) struct Macro {
    pub header: MacroHeader,
    pub body: Vec<MacroBodyLine>,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::preprocessor) enum Sign {
    Plus,
    Minus,
}

impl From<&Token> for Sign {
    fn from(value: &Token) -> Self {
        match &value.kind {
            TokenKind::Plus => Self::Plus,
            TokenKind::Minus => Self::Minus,
            _ => unreachable!(),
        }
    }
}

impl From<&Sign> for &str {
    fn from(value: &Sign) -> Self {
        match value {
            Sign::Plus => "+",
            Sign::Minus => "-",
        }
    }
}

/// Operando simples aceito em diretivas do pré-processador.
///
/// Usado em diretivas que carregam expressão simples (`EQU` e `IF`), onde o
/// valor pode precisar de interpretação semântica (ex.: parsing de literal
/// numérico em base decimal/hex/bin).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Number {
    /// Literal precedido por sinal explícito (`+N` ou `-N`).
    Signed { sign: Sign, sym: Symbol },
    /// Literal sem sinal explícito (`N`).
    Unsigned { sym: Symbol },
}

impl Number {
    /// Retorna o símbolo internado associado ao literal.
    pub(crate) fn sym(&self) -> Symbol {
        match self {
            Self::Signed { sym, .. } | Self::Unsigned { sym } => *sym,
        }
    }
    /// Responde se o número é positivo.
    pub(crate) fn is_positive(&self) -> bool {
        match self {
            Self::Signed {
                sign: Sign::Plus, ..
            } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Operand {
    /// Literal numérico internado (assinado ou não-assinado).
    Number { number: Number, node_id: NodeId },
    /// Identificador internado (nome simbólico/alias).
    Ident { sym: Symbol, node_id: NodeId },
}

impl Operand {
    /// Retorna o `NodeId` do operando.
    pub(crate) fn node_id(&self) -> NodeId {
        match self {
            Self::Number { node_id, .. } | Self::Ident { node_id, .. } => *node_id,
        }
    }
}

/// Argumento posicional de chamada de macro.
///
/// Diferente de [`Operand`], esse tipo representa substituição textual usada
/// na expansão de macro call.
///
/// Intenção de design:
/// - preservar o lexema internado para reemissão como token;
/// - manter rastreabilidade de diagnóstico por `NodeId`.
#[derive(Debug, Clone)]
pub(crate) enum MacroCallArg {
    /// Argumento numérico literal com sinal explícito (`+N` ou `-N`).
    SignedNumber {
        sign: Sign,
        sym: Symbol,
        node_id: NodeId,
    },
    /// Argumento numérico literal sem sinal explícito.
    UnsignedNumber { sym: Symbol, node_id: NodeId },
    /// Argumento identificador.
    Ident { sym: Symbol, node_id: NodeId },
}

impl MacroCallArg {
    /// Retorna o `NodeId` associado ao argumento.
    pub(crate) fn node_id(&self) -> NodeId {
        match self {
            Self::Ident { node_id, .. }
            | Self::UnsignedNumber { node_id, .. }
            | Self::SignedNumber { node_id, .. } => *node_id,
        }
    }

    /// Materializa o argumento como sequência de tokens usando o `span`
    /// informado.
    ///
    /// Regras:
    /// - `Ident` e `UnsignedNumber` geram um único token;
    /// - `SignedNumber` gera dois tokens (`Plus`/`Minus` e `Number`).
    ///
    /// Isso preserva a estrutura léxica esperada pelos próximos parsers
    /// (sinal separado do literal numérico).
    pub(crate) fn to_tokens_with_span(&self, span: Span) -> Vec<Token> {
        match self {
            Self::Ident { sym, .. } => vec![Token::new(TokenKind::Ident(*sym), span)],
            Self::UnsignedNumber { sym, .. } => vec![Token::new(TokenKind::Number(*sym), span)],
            Self::SignedNumber { sign, sym, .. } => {
                let sign_kind = match sign {
                    Sign::Plus => TokenKind::Plus,
                    Sign::Minus => TokenKind::Minus,
                };
                vec![
                    Token::new(sign_kind, span),
                    Token::new(TokenKind::Number(*sym), span),
                ]
            }
        }
    }
}

/// Declaração de seção parseada (`SECTION TEXT` ou `SECTION DATA`).
#[derive(Debug, Clone)]
pub(crate) struct SectionDecl {
    /// `NodeId` da diretiva inteira.
    pub node_id: NodeId,
    /// Seção alvo.
    pub section: Section,
}

/// Declaração `EQU` parseada.
#[derive(Debug, Clone)]
pub(crate) struct EquDecl {
    /// `NodeId` da diretiva inteira.
    pub node_id: NodeId,
    /// Alias definido pela diretiva.
    pub alias: Symbol,
    /// Valor associado ao alias.
    pub value: Operand,
}

/// Declaração `IF` parseada.
#[derive(Debug, Clone)]
pub(crate) struct IfDecl {
    /// `NodeId` da diretiva inteira.
    pub node_id: NodeId,
    /// Condição usada para decidir inclusão/skip da próxima linha.
    pub cond: Operand,
}

/// Chamada de macro parseada.
#[derive(Debug, Clone)]
pub(crate) struct MacroCall {
    /// `NodeId` da chamada inteira.
    pub node_id: NodeId,
    /// Nome da macro chamada.
    pub name: Symbol,
    /// Argumentos posicionais.
    pub args: Vec<MacroCallArg>,
}

/// Linha do corpo de macro em representação estruturada.
///
/// Cada item pode ser literal (token bruto) ou referência a parâmetro formal.
#[derive(Debug, Clone)]
pub(crate) struct MacroBodyLine {
    /// `NodeId` da linha inteira do body.
    pub node_id: NodeId,
    /// Terminador original da linha no momento da definição da macro.
    ///
    /// Normalmente `NewLine`; pode ser sobrescrito na expansão quando a última
    /// linha da macro precisa herdar o terminador da linha de chamada.
    pub terminator: Token,
    pub items: Vec<MacroBodyItem>,
}

/// Unidade semântica de uma linha do corpo de macro.
#[derive(Debug, Clone)]
pub(crate) enum MacroBodyItem {
    /// Token literal preservado como apareceu no body.
    Literal(Token),
    /// Referência a parâmetro formal (ex.: `&ARG`) com `NodeId` próprio.
    ParamRef(Symbol, NodeId),
}
