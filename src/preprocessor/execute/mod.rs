//! Execução semântica das diretivas do pré-processador.
//!
//! Este módulo contém as rotinas que aplicam efeitos no estado interno após a
//! triagem (`detection`) e a validação sintática (`parser`).
//!
//! Organização:
//! - `macro_execute`: utilitários gerais de execução de macro;
//! - `macro_body_execute`: fluxo de acumulação/finalização do body;
//! - `macro_call_execute`: expansão de chamadas e substituição de argumentos;
//! - `equ_execute`: processamento de diretivas `EQU`;
//! - `if_execute`: processamento de diretivas `IF`;
//! - `org_execute`: processamento de `ORG`, incluindo validação semântica de
//!   `Ident` contra a tabela de `EQU`.

mod equ_execute;
mod if_execute;
mod macro_body_execute;
mod macro_call_execute;
mod macro_execute;
mod org_execute;
