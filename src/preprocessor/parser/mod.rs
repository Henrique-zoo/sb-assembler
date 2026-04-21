//! Parser sintático do pré-processador.
//!
//! Este submódulo adota uma abordagem **function-driven**:
//! - cada função resolve uma etapa pequena e bem definida do parsing;
//! - as etapas são compostas em pipeline;
//! - erros são propagados de forma declarativa com `Result` + `?`.
//!
//! Embora não use uma biblioteca formal de parser combinators, o desenho segue
//! esse estilo: funções pequenas "combinam" entre si para construir um parser
//! maior.
//!
//! Exemplo de composição em `parse_macro_header`:
//! 1. `parse_macro_label` extrai a label e o restante da entrada;
//! 2. `consume_required_colon` valida e consome `:`;
//! 3. `consume_macro_keyword` valida e consome `MACRO`;
//! 4. `parse_macro_params` interpreta a lista de parâmetros.
//!
//! Essa organização melhora legibilidade, testabilidade e manutenção:
//! - o comportamento de cada etapa fica isolado;
//! - o fluxo principal vira uma composição linear de transformações;
//! - o primeiro erro relevante interrompe o pipeline com contexto adequado.
//!
//! Organização por assunto:
//! - `macro_parser`: cabeçalho de macro (`<Label>: MACRO ...`);
//! - `macro_body_parser`: sintaxe das linhas do body e `ENDMACRO`;
//! - `macro_call_parser`: sintaxe das chamadas de macro;
//! - `equ_parser`: diretiva `EQU`;
//! - `if_parser`: diretiva `IF`;
//! - `org_parser`: diretiva `ORG` em nível sintático (`ORG <Number|Ident>`),
//!   deixando resolução de `Ident` para `execute`.

mod equ_parser;
mod if_parser;
mod macro_body_parser;
mod macro_call_parser;
mod macro_parser;
mod org_parser;
