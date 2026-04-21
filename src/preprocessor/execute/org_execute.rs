use crate::{
    errors::PreprocessorError,
    interner::Symbol,
    lexer::Token,
    preprocessor::Preprocessor,
};

impl Preprocessor {
    /// Processa uma linha candidata à diretiva `ORG`.
    ///
    /// Fluxo esperado:
    /// 1. chama o parser para obter o operando (`Number` ou `Ident`);
    /// 2. aplica validações semânticas;
    /// 3. efetiva o novo endereço de origem.
    ///
    /// Regra semântica para `Ident`:
    /// - o identificador deve já estar associado a um `EQU` válido no momento
    ///   do uso de `ORG`.
    /// - sob a convenção adotada, isso equivale a exigir que o alias tenha sido
    ///   definido na seção inicial de definições (`MACRO`/`EQU`).
    ///
    /// Essa checagem não pertence ao parser, pois depende de contexto/estado
    /// (`self.equs`).
    pub(in crate::preprocessor) fn process_org(
        &mut self,
        line: &[Token],
    ) -> Result<(), PreprocessorError> {
        let _ = line;
        todo!()
    }

    /// Aplica os efeitos semânticos de um valor de `ORG` já parseado.
    ///
    /// Quando `value` vier de `Ident`, esta etapa resolve o alias na tabela de
    /// `EQU` e falha caso a associação não exista.
    fn apply_org_value(&mut self, value: Symbol) -> Result<(), PreprocessorError> {
        let _ = value;
        todo!()
    }
}
