//! Representação intermediária (IR) do pré-processador.
//!
//! Este módulo reúne os nós sintáticos produzidos pelos parsers do
//! pré-processador. Cada nó representa uma construção já reconhecida e validada
//! sintaticamente, mas ainda sem os efeitos semânticos aplicados ao estado do
//! pré-processador.
//!
//! Papel no pipeline:
//! 1. `detection` identifica, de forma permissiva, se uma linha parece ser uma
//!    diretiva ou chamada relevante para o pré-processador;
//! 2. `parser` valida a forma esperada e converte os tokens em uma IR tipada;
//! 3. `execute` consome essa IR para registrar macros, avaliar `EQU`/`IF`,
//!    trocar seção ou expandir chamadas de macro.
//!
//! Contrato dos tipos:
//! - não executam diretivas e não alteram tabelas semânticas;
//! - preservam símbolos internados (`Symbol`) em vez de copiar strings;
//! - usam `NodeId` para associar nós sintáticos aos spans guardados pelo
//!   `Preprocessor`;
//! - mantêm tokens brutos apenas quando a expansão textual de macro precisa
//!   reemitir a forma léxica original.
//!
//! Escopo:
//! - parsing e diagnóstico de sintaxe ficam nos submódulos `parser`;
//! - avaliação semântica fica nos submódulos `execute`;
//! - este módulo apenas define a forma dos dados que atravessam essa fronteira.

use crate::{
    interner::Symbol,
    language::numeric_literals::NumericLiteral,
    lexer::{Span, Token, TokenKind},
    preprocessor::types::Section,
};

/// Identificador estável de um nó da IR.
///
/// O `NodeId` permite desacoplar metadados de diagnóstico (`Span`) da
/// representação principal da IR. O mapeamento `NodeId -> Span` fica na
/// tabela lateral mantida pelo `Preprocessor`.
///
/// Contrato:
/// - deve ser tratado como identificador opaco;
/// - só é significativo dentro da instância de `Preprocessor` que o alocou;
/// - aponta para o span do nó inteiro, não necessariamente para um token único.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(
    /// Índice na tabela lateral de spans mantida pelo `Preprocessor`.
    ///
    /// O valor não codifica significado semântico próprio; ele apenas permite
    /// recuperar metadados associados ao nó.
    pub u32,
);

/// Alias para parâmetro formal de macro.
///
/// Representa o identificador que aparece depois de `&` em uma declaração de
/// macro.
///
/// Forma canônica:
/// ```ignore
/// ROT: MACRO &A, &B
/// ```
///
/// Neste exemplo, `A` e `B` são armazenados como `Param`.
///
/// Mantido como alias para permitir evolução futura da representação sem
/// alterar assinaturas de alto nível.
pub(crate) type Param = Symbol;

/// Cabeçalho de definição de macro.
///
/// Forma canônica:
/// ```ignore
/// ROT: MACRO &A, &B
/// ```
///
/// Representação:
/// - `name` recebe o símbolo internado de `ROT`;
/// - `params` recebe os símbolos internados de `A` e `B`, sem o prefixo `&`;
/// - `node_id` referencia o span do cabeçalho inteiro.
///
/// Contrato no pipeline:
/// - o parser garante que a label, `:`, `MACRO` e a lista de parâmetros estão
///   sintaticamente válidos;
/// - a etapa de execução usa este nó para registrar a macro na tabela de
///   definições.
#[derive(Debug, Clone)]
pub(crate) struct MacroHeader {
    /// `NodeId` associado ao cabeçalho inteiro.
    ///
    /// O span correspondente cobre a declaração de macro completa, incluindo
    /// rótulo, keyword `MACRO` e lista de parâmetros formais.
    pub node_id: NodeId,
    /// Nome da macro definida.
    ///
    /// Corresponde ao rótulo que antecede `MACRO` na forma canônica:
    ///
    /// ```ignore
    /// NOME: MACRO &ARG
    /// ```
    pub name: Symbol,
    /// Parâmetros formais aceitos pela macro.
    ///
    /// Cada entrada guarda apenas o identificador internado, sem o `&`. O
    /// prefixo é validado pelo parser e não faz parte do símbolo semântico do
    /// parâmetro.
    pub params: Vec<Param>,
}

/// Definição completa de macro armazenada na tabela de macros.
///
/// Uma macro só entra nessa forma depois que o cabeçalho foi validado e o bloco
/// até `ENDMACRO` foi coletado.
///
/// Contrato no pipeline:
/// - `header` identifica nome e parâmetros formais;
/// - `body` preserva a estrutura textual que será reemitida durante a expansão;
/// - a execução da macro substitui referências a parâmetros por argumentos da
///   chamada sem reparsear a definição inteira.
#[derive(Debug, Clone)]
pub(crate) struct Macro {
    /// Cabeçalho que identifica a macro e seus parâmetros formais.
    pub header: MacroHeader,
    /// Linhas do corpo da macro, já separadas em itens literais e referências
    /// a parâmetros.
    ///
    /// O corpo permanece em uma forma próxima dos tokens originais porque a
    /// expansão precisa reemitir código assembly preprocessado, não uma IR de
    /// montagem final.
    pub body: Vec<MacroBodyLine>,
}

/// Sinal explícito associado a um literal numérico.
///
/// O lexer emite `+`/`-` como tokens independentes. Este enum normaliza esse
/// prefixo quando um parser de diretiva reconhece a forma assinada de um
/// número.
///
/// Forma canônica:
/// ```ignore
/// IF +1
/// IF -2
/// ```
///
/// O sinal representa apenas a informação sintática do prefixo. A conversão do
/// literal completo para número da arquitetura acontece em etapa semântica.
#[derive(Debug, Clone, Copy)]
pub(in crate::preprocessor) enum Sign {
    /// Prefixo `+`.
    Plus,
    /// Prefixo `-`.
    Minus,
}

/// Converte um token de sinal já validado em [`Sign`].
///
/// Pré-condição:
/// - `value.kind` deve ser [`TokenKind::Plus`] ou [`TokenKind::Minus`].
///
/// O parser só chama essa conversão depois de reconhecer sintaticamente uma
/// forma assinada. Qualquer outro token representa erro de chamada interna e
/// dispara `unreachable!()`.
impl From<&Token> for Sign {
    fn from(value: &Token) -> Self {
        match &value.kind {
            TokenKind::Plus => Self::Plus,
            TokenKind::Minus => Self::Minus,
            _ => unreachable!(),
        }
    }
}

/// Materializa [`Sign`] como lexema textual.
///
/// Usado quando um valor assinado precisa voltar para uma forma tokenizável,
/// por exemplo durante a expansão de macro ou parsing semântico de números.
impl From<&Sign> for &str {
    fn from(value: &Sign) -> Self {
        match value {
            Sign::Plus => "+",
            Sign::Minus => "-",
        }
    }
}

/// Operando simples aceito por diretivas do pré-processador.
///
/// Usado nas diretivas que recebem uma expressão de um único item, como:
///
/// ```ignore
/// NAME EQU 1
/// IF NAME
/// ```
///
/// Contrato no pipeline:
/// - `Number` representa literal imediato ainda em forma sintática
///   ([`NumericLiteral`]);
/// - `Ident` representa símbolo que deverá ser resolvido pela etapa semântica;
/// - cada variante carrega um `NodeId` próprio para diagnóstico preciso do
///   operando, separado do span da diretiva inteira.
#[derive(Debug, Clone)]
pub(crate) enum Operand {
    /// Literal numérico internado.
    Number {
        /// Valor numérico ainda em forma sintática.
        ///
        /// A conversão para [`crate::assembler::Word`] ou
        /// [`crate::assembler::SignedWord`] acontece no estágio semântico que
        /// consome a diretiva.
        number: NumericLiteral,
        /// `NodeId` associado ao operando inteiro.
        node_id: NodeId,
    },
    /// Identificador internado.
    Ident {
        /// Símbolo que deverá ser resolvido como alias ou condição nomeada pela
        /// etapa semântica.
        sym: Symbol,
        /// `NodeId` associado ao identificador usado como operando.
        node_id: NodeId,
    },
}

/// Argumento posicional de chamada de macro.
///
/// Diferente de [`Operand`], esse tipo representa substituição textual usada
/// na expansão de macro call.
///
/// Forma canônica:
/// ```ignore
/// ROT A, +1, LABEL
/// ```
///
/// Contrato no pipeline:
/// - argumentos são preservados em forma reemitível, pois a expansão de macro
///   substitui parâmetros por tokens;
/// - sinais explícitos continuam separados do literal numérico, mantendo o
///   contrato léxico esperado pelos próximos parsers;
/// - `node_id` permite diagnosticar o argumento original da chamada.
#[derive(Debug, Clone)]
pub(crate) enum MacroCallArg {
    /// Argumento numérico literal com sinal explícito (`+N` ou `-N`).
    SignedNumber {
        /// Sinal textual informado na chamada.
        sign: Sign,
        /// Símbolo internado do literal numérico sem o sinal.
        sym: Symbol,
        /// `NodeId` associado ao argumento inteiro.
        node_id: NodeId,
    },
    /// Argumento numérico literal sem sinal explícito.
    UnsignedNumber {
        /// Símbolo internado do literal numérico.
        sym: Symbol,
        /// `NodeId` associado ao argumento.
        node_id: NodeId,
    },
    /// Argumento identificador.
    Ident {
        /// Símbolo internado do identificador usado como argumento.
        sym: Symbol,
        /// `NodeId` associado ao argumento.
        node_id: NodeId,
    },
}

impl MacroCallArg {
    /// Retorna o `NodeId` associado ao argumento.
    ///
    /// Esse helper permite que a etapa semântica trate todas as formas de
    /// argumento de modo uniforme ao produzir diagnósticos.
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
    /// Regras de emissão:
    /// - `Ident` e `UnsignedNumber` geram um único token;
    /// - `SignedNumber` gera dois tokens (`Plus`/`Minus` e `Number`).
    ///
    /// O `span` recebido é aplicado aos tokens reemitidos. Na expansão de
    /// macro, isso permite associar o trecho gerado à chamada que originou a
    /// substituição.
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

/// Declaração de seção parseada.
///
/// Formas canônicas:
/// ```ignore
/// SECTION TEXT
/// SECTION DATA
/// ```
///
/// Contrato no pipeline:
/// - o parser garante que a diretiva possui exatamente `SECTION` seguido da
///   seção esperada;
/// - a execução altera o contexto de roteamento do preprocessador;
/// - a diretiva não é emitida no [`crate::preprocessor::types::PreprocessedProgram`].
#[derive(Debug, Clone)]
pub(crate) struct SectionDecl {
    /// Seção alvo que deve se tornar ativa.
    pub section: Section,
}

/// Declaração `EQU` parseada.
///
/// Forma canônica:
/// ```ignore
/// NAME EQU VALUE
/// ```
///
/// Contrato no pipeline:
/// - `alias` é o identificador definido pela diretiva;
/// - `value` é mantido como [`Operand`] até a execução resolver número ou
///   identificador;
/// - diagnósticos semânticos usam o `NodeId` do valor, que é o ponto exato de
///   falha;
/// - a execução registra o alias na tabela de `EQU`.
#[derive(Debug, Clone)]
pub(crate) struct EquDecl {
    /// Símbolo internado do alias definido pela diretiva.
    pub alias: Symbol,
    /// Valor sintático associado ao alias.
    pub value: Operand,
}

/// Declaração `IF` parseada.
///
/// Forma canônica:
/// ```ignore
/// IF COND
/// ```
///
/// Contrato no pipeline:
/// - `cond` pode ser número literal ou identificador definido por `EQU`;
/// - a execução resolve a condição e decide se a próxima linha lógica será
///   mantida ou descartada;
/// - este nó não guarda a linha condicionada, apenas a expressão da condição.
#[derive(Debug, Clone)]
pub(crate) struct IfDecl {
    /// `NodeId` da diretiva inteira.
    pub node_id: NodeId,
    /// Condição usada para decidir inclusão ou descarte da próxima linha.
    pub cond: Operand,
}

/// Chamada de macro parseada.
///
/// Forma canônica:
/// ```ignore
/// NAME ARG1, ARG2
/// ```
///
/// Contrato no pipeline:
/// - `name` identifica a macro a buscar na tabela de definições;
/// - `args` preserva os argumentos na ordem da chamada;
/// - a execução valida aridade e expande o corpo da macro para linhas lógicas
///   comuns.
#[derive(Debug, Clone)]
pub(crate) struct MacroCall {
    /// `NodeId` da chamada inteira.
    pub node_id: NodeId,
    /// Símbolo internado do nome da macro chamada.
    pub name: Symbol,
    /// Argumentos posicionais informados na chamada.
    pub args: Vec<MacroCallArg>,
}

/// Linha do corpo de macro em representação estruturada.
///
/// Forma canônica dentro de um bloco de macro:
/// ```ignore
/// LOAD &SRC
/// COPY &FROM, &TO
/// ```
///
/// Contrato no pipeline:
/// - a linha guarda seu terminador original para preservar quebras na emissão;
/// - `items` separa tokens literais de referências a parâmetros formais;
/// - a expansão percorre os itens em ordem e materializa uma nova
///   [`crate::preprocessor::types::LogicalLine`].
#[derive(Debug, Clone)]
pub(crate) struct MacroBodyLine {
    /// `NodeId` da linha inteira do corpo.
    pub node_id: NodeId,
    /// Terminador original da linha no momento da definição da macro.
    ///
    /// Normalmente `NewLine`; pode ser sobrescrito na expansão quando a última
    /// linha da macro precisa herdar o terminador da linha de chamada.
    pub terminator: Token,
    /// Itens sintáticos que compõem a linha do corpo.
    ///
    /// Literais são reemitidos como tokens; referências a parâmetros são
    /// substituídas pelos argumentos correspondentes durante a expansão.
    pub items: Vec<MacroBodyItem>,
}

/// Unidade semântica de uma linha do corpo de macro.
///
/// Durante o parsing do corpo, cada trecho da linha é classificado como token
/// literal ou referência a parâmetro formal. Essa distinção permite que a
/// expansão substitua apenas `&PARAM`, preservando todo o restante da linha.
#[derive(Debug, Clone)]
pub(crate) enum MacroBodyItem {
    /// Token literal preservado como apareceu no corpo da macro.
    ///
    /// O payload é reemitido sem substituição durante a expansão. Ele pode ser
    /// qualquer token que não tenha sido classificado como referência a
    /// parâmetro formal.
    Literal(Token),
    /// Referência a parâmetro formal.
    ///
    /// Campos:
    /// - `Symbol`: identificador internado do parâmetro, sem o prefixo `&`;
    /// - `NodeId`: span da referência inteira no corpo da macro.
    ParamRef(Symbol, NodeId),
}
