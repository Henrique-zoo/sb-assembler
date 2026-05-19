//! Execução semântica de chamadas de macro.
//!
//! Este módulo recebe uma [`crate::preprocessor::ir::MacroCall`] já validada
//! sintaticamente e produz as linhas lógicas expandidas da macro.
//!
//! Responsabilidades:
//! - resolver a definição da macro na tabela interna;
//! - validar aridade (`params` x `args`);
//! - materializar o body substituindo `ParamRef` por argumentos reais;
//! - montar linhas lógicas reutilizando o terminador armazenado no body.
//!
//! Observação arquitetural:
//! - o parser (`parser/macro_call_parser.rs`) garante forma;
//! - este módulo resolve significado e expansão textual.

use std::collections::HashMap;

use crate::{
    errors::{MacroCallSemanticErrorKind, PreprocessorError, PreprocessorErrorKind},
    interner::Symbol,
    lexer::{Span, Token},
    preprocessor::{
        Preprocessor,
        ir::{Macro, MacroBodyItem, MacroCall, MacroCallArg},
        types::LogicalLine,
    },
};

impl Preprocessor<'_> {
    /// Expande uma chamada de macro já parseada para linhas lógicas.
    ///
    /// Forma canônica que chega aqui (já validada no parser):
    /// ```ignore
    /// <MacroName>
    /// <MacroName> <Arg1>, <Arg2>, ...
    /// ```
    ///
    /// Fluxo:
    /// 1. resolve a definição com [`Self::resolve_macro_definition`];
    /// 2. valida aridade (quantidade de argumentos);
    /// 3. cria bindings `parametro formal -> argumento real` com
    ///    [`Self::build_macro_call_bindings`];
    /// 4. materializa o body substituindo `ParamRef` por tokens concretos com
    ///    [`Self::expand_macro_body`].
    ///
    /// Retorno:
    /// - `Ok(Vec<LogicalLine>)` com as linhas expandidas da chamada.
    ///
    /// Erros:
    /// - `UndefinedMacro` se o nome não existe na tabela;
    /// - `WrongArgCount` se a aridade não coincide;
    /// - `MissingArguments` se faltar binding para algum `ParamRef` no body.
    ///
    /// Efeito colateral:
    /// - nenhum. A função não altera `self`; apenas consulta tabelas internas.
    pub(in crate::preprocessor) fn execute_macro_call(
        &self,
        macro_call: MacroCall,
    ) -> Result<Vec<LogicalLine>, PreprocessorError> {
        let macro_def = self.resolve_macro_definition(&macro_call)?;
        self.ensure_macro_call_arity(macro_def, &macro_call)?;
        let bindings = Self::build_macro_call_bindings(&macro_def.header.params, &macro_call.args);
        self.expand_macro_body(macro_def, &bindings)
    }

    /// Resolve a definição da macro chamada.
    ///
    /// Retorna a macro registrada em `self.macros` para o `name` da chamada.
    ///
    /// Erros:
    /// - `InvalidMacroCallSemantic(UndefinedMacro)` quando o nome da chamada
    ///   não existe na tabela de macros.
    ///
    /// Diagnóstico:
    /// - usa o `NodeId` da própria chamada para recuperar o `span` do erro.
    fn resolve_macro_definition<'a>(
        &'a self,
        macro_call: &MacroCall,
    ) -> Result<&'a Macro, PreprocessorError> {
        self.macros.get(&macro_call.name).ok_or_else(|| {
            Self::macro_call_semantic_error(
                MacroCallSemanticErrorKind::UndefinedMacro,
                self.span_of_node(macro_call.node_id),
            )
        })
    }

    /// Garante que a quantidade de argumentos da chamada coincide com a
    /// quantidade de parâmetros formais da definição.
    ///
    /// Regra:
    /// - `macro_call.args.len()` deve ser igual a
    ///   `macro_def.header.params.len()`.
    ///
    /// Erros:
    /// - `InvalidMacroCallSemantic(WrongArgCount { expected, found })` quando
    ///   a aridade diverge.
    fn ensure_macro_call_arity(
        &self,
        macro_def: &Macro,
        macro_call: &MacroCall,
    ) -> Result<(), PreprocessorError> {
        let expected = macro_def.header.params.len();
        let found = macro_call.args.len();

        if found != expected {
            let span = macro_call
                .args
                .get(expected)
                .map(|arg| self.span_of_node(arg.node_id()))
                .unwrap_or_else(|| self.span_of_node(macro_call.node_id));

            return Err(Self::macro_call_semantic_error(
                MacroCallSemanticErrorKind::WrongArgCount { expected, found },
                span,
            ));
        }

        Ok(())
    }

    /// Cria o mapa `parametro formal -> argumento real` da chamada.
    ///
    /// A estrutura guarda referências para evitar cópia dos operandos.
    ///
    /// Pré-condição:
    /// - a aridade já deve ter sido validada antes (via
    ///   [`Self::ensure_macro_call_arity`]).
    ///
    /// Observação:
    /// - o `zip` preserva ordem posicional: `params[i] -> args[i]`.
    fn build_macro_call_bindings<'a>(
        params: &[Symbol],
        args: &'a [MacroCallArg],
    ) -> HashMap<Symbol, &'a MacroCallArg> {
        params.iter().copied().zip(args.iter()).collect()
    }

    /// Expande o body da macro substituindo referências a parâmetros.
    ///
    /// Cada `MacroBodyLine` gera uma `LogicalLine` reaproveitando o terminador
    /// armazenado na própria IR da linha.
    ///
    /// Contrato:
    /// - a ordem das linhas do body é preservada;
    /// - cada item da linha é expandido por [`Self::expand_macro_body_item`];
    /// - o terminador é herdado de `MacroBodyLine::terminator`.
    ///
    /// Integração:
    /// - o orquestrador (`process`) pode ajustar o terminador da última
    ///   linha expandida para reaproveitar o terminador original da linha de
    ///   chamada.
    fn expand_macro_body(
        &self,
        macro_def: &Macro,
        bindings: &HashMap<Symbol, &MacroCallArg>,
    ) -> Result<Vec<LogicalLine>, PreprocessorError> {
        macro_def
            .body
            .iter()
            .map(|line| {
                let content = line
                    .items
                    .iter()
                    .try_fold(Vec::new(), |mut content, item| {
                        content.extend(self.expand_macro_body_item(item, bindings)?);
                        Ok::<Vec<Token>, PreprocessorError>(content)
                    })?;

                Ok(LogicalLine {
                    content,
                    terminator: line.terminator,
                })
            })
            .collect::<Result<Vec<LogicalLine>, PreprocessorError>>()
    }

    /// Expande um item único de body de macro.
    ///
    /// - `Literal(tok)` é preservado sem alteração.
    /// - `ParamRef(sym, node_id)` é materializado como sequência de tokens do
    ///   argumento correspondente, preservando o `Span` associado ao `node_id`.
    ///
    /// Erros:
    /// - `InvalidMacroCallSemantic(MissingArguments)` quando não há binding
    ///   para o parâmetro referenciado.
    ///
    /// Observação de diagnóstico:
    /// - o `span` do erro (e dos tokens materializados) é o `span` do ponto de
    ///   uso do parâmetro no body da macro, não o `span` do argumento na call.
    fn expand_macro_body_item(
        &self,
        item: &MacroBodyItem,
        bindings: &HashMap<Symbol, &MacroCallArg>,
    ) -> Result<Vec<Token>, PreprocessorError> {
        match item {
            MacroBodyItem::Literal(tok) => Ok(vec![*tok]),
            MacroBodyItem::ParamRef(param_sym, param_ref_node_id) => {
                let arg = bindings.get(param_sym).ok_or_else(|| {
                    Self::macro_call_semantic_error(
                        MacroCallSemanticErrorKind::MissingArguments,
                        self.span_of_node(*param_ref_node_id),
                    )
                })?;

                Ok(arg.to_tokens_with_span(self.span_of_node(*param_ref_node_id)))
            }
        }
    }

    /// Encapsula um erro da família `MacroCall` preservando o `span`.
    ///
    /// Este helper centraliza a construção de
    /// [`crate::errors::PreprocessorErrorKind::InvalidMacroCallSemantic`].
    fn macro_call_semantic_error(
        kind: MacroCallSemanticErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError {
            kind: PreprocessorErrorKind::InvalidMacroCallSemantic(kind),
            span,
        }
    }
}
