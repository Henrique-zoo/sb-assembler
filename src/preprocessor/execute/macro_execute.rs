use crate::{
    errors::{PreprocessorError, PreprocessorErrorKind},
    lexer::Span,
    preprocessor::{
        LogicalLineIter, Preprocessor,
        ir::{Macro, MacroBodyLine, MacroHeader, NodeId},
    },
};

impl Preprocessor {
    /// Aplica os efeitos semânticos de um cabeçalho `MACRO` já parseado.
    ///
    /// A função recebe o iterador de linhas lógicas e consome o bloco da macro
    /// até encontrar uma linha candidata a `ENDMACRO`.
    ///
    /// Durante o consumo:
    /// - linhas intermediárias são acumuladas no body da macro.
    ///
    /// Quando o iterador acaba antes de `ENDMACRO`, a definição é considerada
    /// inválida e a função retorna `UnterminatedMacro`.
    pub(in crate::preprocessor) fn execute_macro_header(
        &mut self,
        macro_header: MacroHeader,
        lines: &mut LogicalLineIter,
    ) -> Result<(), PreprocessorError> {
        let body = self.collect_macro_block(macro_header.node_id, lines)?;
        self.register_macro_definition(macro_header, body)?;
        Ok(())
    }

    /// Coleta o bloco de uma macro até `ENDMACRO`.
    ///
    /// Fluxo:
    /// 1. consome linhas do iterador;
    /// 2. quando encontra candidata a `ENDMACRO`, valida a linha e encerra;
    /// 3. para linhas de conteúdo não vazias, parseia e acumula no body.
    ///
    /// Retorno:
    /// - `Ok(Vec<MacroBodyLine>)` com as linhas parseadas da definição.
    ///
    /// Erros:
    /// - propaga erro sintático de `ENDMACRO` inválido;
    /// - propaga erro sintático de parsing de uma linha de body;
    /// - `UnterminatedMacro` quando o iterador acaba sem `ENDMACRO`.
    ///
    /// Efeito colateral:
    /// - avança `lines` consumindo toda a definição da macro.
    fn collect_macro_block(
        &mut self,
        header_node_id: NodeId,
        lines: &mut LogicalLineIter,
    ) -> Result<Vec<MacroBodyLine>, PreprocessorError> {
        let mut body = Vec::new();
        let mut unterminated_span = self.span_of_node(header_node_id);

        while let Some(logical_line) = lines.next() {
            if self.looks_like_endmacro_line(&logical_line.content) {
                self.parse_endmacro_line(&logical_line.content)?;
                return Ok(body);
            }

            if logical_line.content.first().is_some() {
                let parsed_line =
                    self.parse_macro_body_line(&logical_line.content, logical_line.terminator)?;
                unterminated_span = self.span_of_node(parsed_line.node_id);
                body.push(parsed_line);
            }
        }

        Err(Self::macro_definition_semantic_error(
            PreprocessorErrorKind::UnterminatedMacro,
            unterminated_span,
        ))
    }

    /// Registra uma macro definida no estado interno.
    ///
    /// Regras:
    /// - rejeita redefinição quando o nome já existe em `self.macros`;
    /// - grava `name -> Macro { header, body }` quando o nome é novo.
    ///
    /// Erros:
    /// - `MacroAlreadyDefined` quando a macro já foi registrada.
    fn register_macro_definition(
        &mut self,
        macro_header: MacroHeader,
        body: Vec<MacroBodyLine>,
    ) -> Result<(), PreprocessorError> {
        if self.macros.contains_key(&macro_header.name) {
            return Err(Self::macro_definition_semantic_error(
                PreprocessorErrorKind::MacroAlreadyDefined(macro_header.name),
                self.span_of_node(macro_header.node_id),
            ));
        }

        self.macros.insert(
            macro_header.name,
            Macro {
                header: macro_header,
                body,
            },
        );

        Ok(())
    }

    /// Constrói erro associado ao fluxo de definição de macro, preservando span.
    fn macro_definition_semantic_error(
        kind: PreprocessorErrorKind,
        span: Span,
    ) -> PreprocessorError {
        PreprocessorError { kind, span }
    }
}
