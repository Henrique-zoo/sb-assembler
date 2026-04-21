//! Detecção heurística de diretivas do pré-processador.
//!
//! Este módulo reúne detectores `looks_like_*` cuja responsabilidade é
//! **triagem**: responder se uma linha *parece* uma tentativa de uso de
//! diretiva (`MACRO`, `ENDMACRO`, `EQU`, `IF`, `ORG`), sem exigir sintaxe
//! estrita.
//!
//! ## Objetivo principal
//!
//! A detecção é propositalmente permissiva para não deixar passar erros de
//! autoria. Quando uma linha tenta declarar uma diretiva, mas está malformada,
//! o pré-processador deve conseguir:
//! 1. reconhecer que há uma tentativa de diretiva;
//! 2. encaminhar a linha para parsing/validação;
//! 3. emitir um erro sintático específico em vez de tratar a linha como código
//!    comum.
//!
//! Em outras palavras: este módulo evita falsos negativos de intenção.
//!
//! ## Por que a permissividade é desejável
//!
//! Exemplo: `X: EQU 10`.
//! - O detector de `EQU` pode retornar `true` (há clara tentativa de diretiva).
//! - O parser de `EQU`, que é estrito, rejeita a forma com `:`.
//! - O usuário recebe diagnóstico correto sobre a sintaxe da diretiva.
//!
//! Se a detecção fosse estrita demais, essa linha poderia ser ignorada como
//! diretiva e cair no fluxo normal, piorando a qualidade do erro.
//!
//! ## Limites de responsabilidade
//!
//! Este módulo **não**:
//! - valida gramática completa;
//! - decide semântica/efeitos da diretiva;
//! - produz diagnósticos detalhados de sintaxe.
//!
//! Esses papéis pertencem ao módulo [`crate::preprocessor::parser`].
//!
//! ## Integração com o restante do pré-processador
//!
//! Fluxo resumido:
//! 1. `process_line` (orquestração) chama `looks_like_*`;
//! 2. se `true`, delega ao parser apropriado;
//! 3. parser valida estrutura e extrai dados;
//! 4. estágio de execução aplica efeito ou acumula erro.
//!
//! O detalhamento completo desse caminho pertence ao módulo-orquestrador
//! [`crate::preprocessor`] (onde o fluxo de estados e despacho acontece).
//!
//! ## Organização interna
//!
//! Os detectores estão separados por diretiva:
//! - `macro_detection`: `MACRO` e `ENDMACRO`;
//! - `equ_detection`: `EQU`;
//! - `if_detection`: `IF`.
//! - `org_detection`: `ORG`.
//!
//! Além disso, este módulo separa fluxos específicos de macro:
//! - `macro_body_detection`: tentativas relacionadas ao corpo da macro
//!   (linhas internas e encerramento);
//! - `macro_call_detection`: tentativas de chamada de macro.

mod equ_detection;
mod if_detection;
mod macro_body_detection;
mod macro_call_detection;
mod macro_detection;
mod org_detection;
