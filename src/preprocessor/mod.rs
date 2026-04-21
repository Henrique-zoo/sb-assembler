//! Orquestração do pré-processador.
//!
//! Este módulo coordena o pipeline de pré-processamento por linha:
//! 1. triagem com detectores (`detection::looks_like_*`);
//! 2. validação sintática/extração (`parser::*`);
//! 3. aplicação de efeitos (`execute::*`: registro de macro/EQU, expansão, controle de
//!    fluxo, ORG) e acumulação de erros.
//!
//! Separação de responsabilidades:
//! - [`crate::preprocessor::detection`]: reconhecimento permissivo de tentativa
//!   de diretiva;
//! - [`crate::preprocessor::parser`]: validação sintática estrita e parsing
//!   (ex.: `ORG` aceita `Number` ou `Ident`);
//! - [`crate::preprocessor::execute`]: execução semântica das diretivas;
//! - este módulo: despacho por estado e montagem do output.

mod detection;
mod execute;
mod parser;
mod types;

pub(crate) use types::{Keywords, Macro, Preprocessor, State};

use std::collections::HashMap;

use crate::{
    errors::{PreprocessorError, PreprocessorErrorKind},
    interner::Interner,
    lexer::{Token, TokenKind},
};
use types::PreprocessorAcc;

impl Preprocessor {
    /// Cria uma instância de `Preprocessor` pronta para uso.
    ///
    /// A construção inicializa o estado interno como `State::Normal`, cria
    /// tabelas vazias para macros e `EQU`, e interna as keywords do
    /// pré-processador para comparações eficientes por símbolo.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let preprocessor = Preprocessor::new(&mut interner);
    /// ```
    pub fn new(interner: &mut Interner) -> Self {
        let keywords = Keywords {
            macro_kw: interner.entry("MACRO").or_insert(),
            endmacro_kw: interner.entry("ENDMACRO").or_insert(),
            equ_kw: interner.entry("EQU").or_insert(),
            if_kw: interner.entry("IF").or_insert(),
            org_kw: interner.entry("ORG").or_insert(),
        };

        Self {
            macros: HashMap::new(),
            equs: HashMap::new(),
            state: State::Normal,
            keywords,
        }
    }

    /// Executa o pré-processamento sobre o fluxo de tokens produzido pelo lexer.
    ///
    /// A função percorre os tokens, agrupa o conteúdo por linhas lógicas e
    /// delega cada linha para `process_line`. Os separadores originais
    /// (`NewLine` e `Eof`) são preservados no resultado.
    ///
    /// O processamento não interrompe no primeiro problema: erros são
    /// acumulados e devolvidos juntos no final. Durante esse fluxo, o
    /// pré-processador pode alterar estado, registrar diretivas e expandir
    /// macros.
    ///
    /// Ao término, é feita uma verificação final para detectar macro aberta
    /// sem `ENDMACRO`, gerando `UnterminatedMacro` quando aplicável.
    ///
    /// Retorna `Ok(output)` quando nenhum erro foi acumulado e `Err(errors)`
    /// quando há um ou mais diagnósticos.
    ///
    /// # Exemplo
    /// ```rust,ignore
    /// let mut interner = Interner::new();
    /// let mut preprocessor = Preprocessor::new(&mut interner);
    ///
    /// let add = interner.entry("ADD").or_insert();
    /// let tokens = vec![
    ///     Token::new(TokenKind::Ident(add), Span { pos: 0, line: 1, column: 1, len: 3 }),
    ///     Token::new(TokenKind::NewLine, Span { pos: 3, line: 1, column: 4, len: 1 }),
    ///     Token::new(TokenKind::Eof, Span { pos: 4, line: 2, column: 1, len: 0 }),
    /// ];
    ///
    /// let result = preprocessor.process(tokens, &mut interner);
    /// assert!(result.is_ok());
    /// ```
    pub fn process(
        &mut self,
        tokens: Vec<Token>,
        interner: &mut Interner,
    ) -> Result<Vec<Token>, Vec<PreprocessorError>> {
        let PreprocessorAcc {
            output,
            current_line: _,
            mut errors,
        } = tokens
            .into_iter()
            .fold(PreprocessorAcc::new(), |mut acc, token| {
                if matches!(token.kind, TokenKind::NewLine | TokenKind::Eof) {
                    self.process_line(
                        &acc.current_line,
                        &mut acc.output,
                        &mut acc.errors,
                        interner,
                    );

                    acc.output.push(token);
                } else {
                    acc.current_line.push(token);
                }

                acc
            });

        // Todos os tokens já foram processados, logo, se o estado aqui é DefiningMacro, algum macro não foi fechado
        if let State::DefiningMacro { body, .. } = &self.state {
            let span = body
                .last()
                .and_then(|line| line.first())
                .map(|t| t.span)
                .unwrap_or(crate::lexer::Span {
                    pos: 0,
                    line: 1,
                    column: 1,
                    len: 0,
                });

            errors.push(PreprocessorError {
                kind: PreprocessorErrorKind::UnterminatedMacro,
                span,
            });
        }

        if errors.is_empty() {
            Ok(output)
        } else {
            Err(errors)
        }
    }

    /// Processa uma linha lógica (sem `NewLine`/`Eof`) de acordo com o estado
    /// atual do pré-processador.
    ///
    /// Em `State::Normal`, a função usa os detectores `looks_like_*` como
    /// triagem
    /// rápida e delega a validação/aplicação da diretiva para as rotinas
    /// correspondentes. Se a linha não representar diretiva nem expansão de
    /// macro, ela é repassada para `output` sem alterações.
    ///
    /// Em `State::DefiningMacro`, as linhas são acumuladas no corpo da macro
    /// até que um fechamento seja encontrado.
    ///
    /// Essa função pode alterar estado interno, atualizar tabelas, escrever em
    /// `output` e acumular erros em `errors`. Linhas vazias retornam cedo.
    ///
    /// # Exemplo (uso interno)
    /// ```rust,ignore
    /// let mut output = Vec::new();
    /// let mut errors = Vec::new();
    /// preprocessor.process_line(&line, &mut output, &mut errors, &mut interner);
    /// ```
    fn process_line(
        &mut self,
        line: &[Token],
        output: &mut Vec<Token>,
        errors: &mut Vec<PreprocessorError>,
        interner: &mut Interner,
    ) {
        if line.is_empty() {
            return;
        }
        let is_endmacro_line = self.looks_like_endmacro_line(line);

        match &mut self.state {
            State::Normal => {
                if self.looks_like_macro_header(line) {
                    match self.parse_macro_header(line) {
                        Ok(_) => self.execute_macro_header(),
                        Err(e) => errors.push(e),
                    }
                    return;
                }

                if self.looks_like_equ_line(line) {
                    if let Err(err) = self.process_equ(line) {
                        errors.push(err);
                    }
                    return;
                }

                if self.looks_like_if_line(line) {
                    if let Err(err) = self.process_if(line, output) {
                        errors.push(err);
                    }
                    return;
                }

                if self.looks_like_org_line(line) {
                    if let Err(err) = self.process_org(line) {
                        errors.push(err);
                    }
                    return;
                }

                if let Some(expanded) = self.expand_macro_call(line, interner, errors) {
                    for expanded_line in expanded {
                        output.extend(expanded_line);
                    }
                    return;
                }

                output.extend_from_slice(line);
            }

            State::DefiningMacro { body, .. } => {
                if is_endmacro_line {
                    if let Err(err) = self.finish_macro_definition() {
                        errors.push(err);
                    }
                } else {
                    body.push(line.to_vec());
                }
            }
        }
    }
}
