//! Montagem em uma passagem sobre a IR do parser.
//!
//! Este módulo consome um [`ParsedProgram`] já validado sintaticamente e emite
//! dois artefatos em paralelo:
//! - o `.obj`, com o código final resolvido;
//! - o `.pen`, com o código produzido pelo algoritmo de passagem única e as
//!   listas de pendência embutidas no próprio código.
//!
//! Quando encontra referência a símbolo ainda não definido, o `.obj` recebe uma
//! palavra provisória para remendo posterior, enquanto o `.pen` recebe o elo
//! anterior da lista de pendências daquele símbolo.

use crate::{
    assembler::Word,
    assembler::assembly::one_pass::types::{
        AssemblyArtifacts, Offset, PatchSite, PendingProgram, SymbolEntry, SymbolTable,
    },
    assembler::errors::{AssemblyError, AssemblyErrorKind, NumberContext},
    assembler::interner::{Interner, Symbol},
    assembler::language::{
        instructions::{InstructionSet, Mnemonic},
        numeric_literals::{self, NumberParseError, NumericLiteral},
    },
    assembler::lexer::Span,
    assembler::parser::{
        ir::{AddressOperand, DataDirective, Instruction, LabelDef, NumberLiteral, SymbolRef},
        types::{DataLine, NodeSpans, ParsedProgram, TextLine},
    },
};

pub(crate) mod types;

pub(crate) use types::ObjectProgram;

pub(crate) struct OnePassAssembler<'a> {
    pub interner: &'a Interner,
    pub isa: &'a InstructionSet,

    pub symbols: SymbolTable,
    pub location_counter: usize,
    pub errors: Vec<AssemblyError>,
    pub artifacts: AssemblyArtifacts,
}

impl<'a> OnePassAssembler<'a> {
    /// Cria um montador de uma passagem com estado semântico vazio.
    ///
    /// O montador guarda referências para as tabelas compartilhadas da
    /// linguagem e inicializa o estado mutável usado durante a passagem:
    /// tabela de símbolos, contador de posição, lista de diagnósticos e
    /// artefatos de saída.
    ///
    /// # Parâmetros
    /// - `interner`: tabela usada para converter símbolos internados em
    ///   lexemas durante validação numérica;
    /// - `isa`: especificação de opcodes e tamanhos das instruções.
    ///
    /// # Retorno
    /// - [`OnePassAssembler`] pronto para consumir um
    ///   [`crate::assembler::parser::types::ParsedProgram`].
    pub(crate) fn new(interner: &'a Interner, isa: &'a InstructionSet) -> Self {
        Self {
            interner,
            isa,
            symbols: SymbolTable::new(),
            location_counter: 0,
            errors: Vec::new(),
            artifacts: AssemblyArtifacts {
                object: ObjectProgram { words: Vec::new() },
                pending: PendingProgram { words: Vec::new() },
            },
        }
    }
}

impl<'a> OnePassAssembler<'a> {
    /// Executa a montagem de uma passagem sobre um programa parseado.
    ///
    /// O fluxo percorre primeiro as linhas de `TEXT`, emitindo opcodes e
    /// operandos no `.obj` e no `.pen`. Em seguida percorre `DATA`, define
    /// rótulos de dados, emite diretivas montáveis e remenda no `.obj` os usos
    /// futuros que agora podem ser resolvidos. O `.pen` não é remendado: ele
    /// preserva as listas de pendência no próprio código, conforme a
    /// especificação do trabalho.
    ///
    /// # Parâmetros
    /// - `program`: IR final separada em `text` e `data`, com a tabela lateral
    ///   de spans produzida pelo parser.
    ///
    /// # Retorno
    /// - `Ok(AssemblyArtifacts { object, pending })` quando o `.obj` pôde ser
    ///   resolvido e o `.pen` pôde ser emitido;
    /// - `Err(errors)` quando a passagem acumula um ou mais diagnósticos.
    ///
    /// # Erros
    /// Acumula erros de montagem sem interromper a passagem sempre que possível:
    /// rótulo duplicado, símbolo indefinido, número inválido, estouro numérico,
    /// quantidade inválida de `SPACE`, mnemônico inválido e estouro de endereço.
    pub(crate) fn assemble(
        mut self,
        program: ParsedProgram,
    ) -> Result<AssemblyArtifacts, Vec<AssemblyError>> {
        let ParsedProgram {
            text,
            data,
            node_spans,
        } = program;
        self.assemble_text(text, &node_spans);
        self.assemble_data(data, &node_spans);
        self.report_undefined_symbols();

        if self.errors.is_empty() {
            Ok(self.artifacts)
        } else {
            Err(self.errors)
        }
    }

    /// Emite as linhas da seção `TEXT`.
    ///
    /// Cada linha define seu rótulo opcional no endereço corrente antes de
    /// emitir a instrução. Essa ordem preserva a semântica usual de assembly:
    /// o rótulo aponta para o primeiro word gerado pela instrução da própria
    /// linha.
    ///
    /// # Parâmetros
    /// - `text`: linhas parseadas da seção `TEXT`;
    /// - `node_spans`: tabela lateral usada para diagnósticos.
    ///
    /// # Efeitos colaterais
    /// - adiciona palavras em `self.artifacts.object` e
    ///   `self.artifacts.pending`;
    /// - avança `self.location_counter`;
    /// - registra definições e usos pendentes em `self.symbols`;
    /// - acumula diagnósticos em `self.errors`.
    fn assemble_text(&mut self, text: Vec<TextLine>, node_spans: &NodeSpans) {
        text.into_iter().for_each(|parsed_line| {
            if let Some(label) = parsed_line.label {
                self.define_symbol(label, node_spans);
            }

            self.emit_instruction(parsed_line.body, node_spans);
        });
    }

    /// Emite uma instrução da seção `TEXT`.
    ///
    /// A variante da IR determina quantos operandos de endereço devem ser
    /// emitidos depois do opcode. O mnemônico é validado contra a tabela da ISA
    /// antes de qualquer operando ser escrito.
    ///
    /// # Parâmetros
    /// - `instruction`: instrução parseada;
    /// - `node_spans`: tabela lateral usada para recuperar spans da instrução
    ///   e de seus operandos.
    ///
    /// # Efeitos colaterais
    /// Emite o opcode e seus operandos nos dois artefatos, podendo criar
    /// remendos para o `.obj` e elos de pendência no `.pen`.
    fn emit_instruction(&mut self, instruction: Instruction, node_spans: &NodeSpans) {
        match instruction {
            Instruction::NoOperand { mnemonic, node_id } => {
                let span = node_spans.span_of(node_id);
                self.emit_opcode(mnemonic, span);
            }
            Instruction::OneOperand {
                mnemonic,
                operand,
                node_id,
            } => {
                let span = node_spans.span_of(node_id);
                if self.emit_opcode(mnemonic, span) {
                    self.emit_address_operand(operand, node_spans);
                }
            }
            Instruction::TwoOperands {
                mnemonic,
                first,
                second,
                node_id,
            } => {
                let span = node_spans.span_of(node_id);
                if self.emit_opcode(mnemonic, span) {
                    self.emit_address_operand(first, node_spans);
                    self.emit_address_operand(second, node_spans);
                }
            }
        }
    }

    /// Emite o opcode associado a um mnemônico.
    ///
    /// # Parâmetros
    /// - `mnemonic`: mnemônico já reconhecido pelo parser;
    /// - `span`: região da instrução usada caso a ISA não contenha o mnemônico.
    ///
    /// # Retorno
    /// - `true` quando o opcode foi encontrado e emitido;
    /// - `false` quando o mnemônico não existe na ISA.
    ///
    /// # Erros
    /// Em caso de mnemônico inexistente, acumula
    /// [`AssemblyErrorKind::InvalidMnemonic`] e não emite operandos.
    fn emit_opcode(&mut self, mnemonic: Mnemonic, span: Span) -> bool {
        let Some(spec) = self.isa.specs.get(&mnemonic) else {
            self.errors.push(AssemblyError {
                kind: AssemblyErrorKind::InvalidMnemonic { mnemonic },
                span,
            });
            return false;
        };

        let opcode = spec.opcode;
        self.push_resolved_word(opcode, span);
        true
    }

    /// Emite um operando de endereço.
    ///
    /// Forma canônica consumida pela montagem:
    ///
    /// ```ignore
    /// LOAD VALUE
    /// LOAD TABLE + 2
    /// LOAD TABLE - 1
    /// ```
    ///
    /// # Parâmetros
    /// - `operand`: operando direto ou com deslocamento;
    /// - `node_spans`: tabela lateral usada para recuperar spans.
    ///
    /// # Efeitos colaterais
    /// Emite o endereço resolvido nos dois artefatos quando possível. Quando o
    /// símbolo base ainda não existe, emite placeholder no `.obj` e elo de
    /// lista no `.pen`.
    fn emit_address_operand(&mut self, operand: AddressOperand, node_spans: &NodeSpans) {
        match operand {
            AddressOperand::Direct(symbol_ref) => {
                let span = node_spans.span_of(symbol_ref.node_id);
                self.emit_symbol_address(symbol_ref, 0, span);
            }
            AddressOperand::Offset {
                base,
                offset,
                node_id,
            } => {
                let operand_span = node_spans.span_of(node_id);
                let offset_span = node_spans.span_of(offset.node_id);
                let Some(offset) = self.parse_address_offset(offset.literal, offset_span) else {
                    self.push_resolved_word(0, operand_span);
                    return;
                };

                self.emit_symbol_address(base, offset, operand_span);
            }
        }
    }

    /// Resolve ou posterga a emissão de um endereço simbólico.
    ///
    /// Se o símbolo já foi definido, a função aplica o deslocamento e emite o
    /// endereço final em `.obj` e `.pen`. Se o símbolo ainda não foi definido,
    /// emite `0` como placeholder no `.obj`, emite no `.pen` o endereço do uso
    /// pendente anterior do mesmo símbolo e registra o ponto de remendo.
    ///
    /// # Parâmetros
    /// - `symbol_ref`: referência simbólica usada pelo operando;
    /// - `offset`: deslocamento já convertido para inteiro;
    /// - `span`: região do operando para diagnósticos.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::AddressOverflow`] quando
    /// `address + offset` não cabe em [`Word`].
    fn emit_symbol_address(&mut self, symbol_ref: SymbolRef, offset: Offset, span: Span) {
        let patch = PatchSite {
            output_index: self.artifacts.object.words.len(),
            offset,
            span,
        };

        match self.symbols.entries.remove(&symbol_ref.name) {
            Some(SymbolEntry::Defined {
                address,
                defined_at,
            }) => {
                if let Some(resolved) = Self::checked_address(address, offset) {
                    self.push_resolved_word(resolved, span);
                } else {
                    self.errors.push(AssemblyError {
                        kind: AssemblyErrorKind::AddressOverflow {
                            address: Self::saturating_address_for_diagnostic(address, offset),
                        },
                        span,
                    });
                    self.push_resolved_word(0, span);
                }
                self.symbols.entries.insert(
                    symbol_ref.name,
                    SymbolEntry::Defined {
                        address,
                        defined_at,
                    },
                );
            }
            Some(SymbolEntry::Pending {
                mut uses,
                list_head,
            }) => {
                let previous_head = list_head;
                let new_head = self.artifacts.pending.words.len();
                uses.push(patch);
                self.push_unresolved_address(previous_head, span);
                self.symbols.entries.insert(
                    symbol_ref.name,
                    SymbolEntry::Pending {
                        uses,
                        list_head: Some(new_head),
                    },
                );
            }
            None => {
                let new_head = self.artifacts.pending.words.len();
                self.push_unresolved_address(None, span);
                self.symbols.entries.insert(
                    symbol_ref.name,
                    SymbolEntry::Pending {
                        uses: vec![patch],
                        list_head: Some(new_head),
                    },
                );
            }
        }
    }

    /// Emite as linhas da seção `DATA`.
    ///
    /// A seção de dados participa da mesma contagem de posição usada por
    /// `TEXT`. Por isso, rótulos de dados recebem o endereço corrente após a
    /// emissão do texto, e referências pendentes vindas de instruções podem ser
    /// remendadas assim que o rótulo aparece.
    ///
    /// # Parâmetros
    /// - `data`: linhas parseadas da seção `DATA`;
    /// - `node_spans`: tabela lateral usada para diagnósticos.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::NoRotuleInDataLine`] quando uma linha de
    /// `DATA` não possui rótulo.
    fn assemble_data(&mut self, data: Vec<DataLine>, node_spans: &NodeSpans) {
        for parsed_line in data {
            match parsed_line.label {
                Some(label) => self.define_symbol(label, node_spans),
                None => self.errors.push(AssemblyError {
                    kind: AssemblyErrorKind::NoRotuleInDataLine,
                    span: node_spans.span_of(parsed_line.node_id),
                }),
            }

            self.emit_data_directive(parsed_line.body, node_spans);
        }
    }

    /// Aplica o efeito montável de uma diretiva de dados.
    ///
    /// Formas canônicas:
    ///
    /// ```ignore
    /// VALUE CONST 10
    /// BUFFER SPACE
    /// TABLE SPACE 4
    /// ```
    ///
    /// # Parâmetros
    /// - `directive`: diretiva `CONST` ou `SPACE` já parseada;
    /// - `node_spans`: tabela lateral usada para recuperar spans.
    ///
    /// # Efeitos colaterais
    /// - `CONST` emite uma palavra em `self.artifacts`;
    /// - `SPACE` emite uma ou mais palavras zeradas nos dois artefatos;
    /// - ambas avançam `self.location_counter` pelas palavras emitidas.
    ///
    /// # Erros
    /// Acumula erros numéricos ou quantidade inválida de `SPACE`. Quando uma
    /// conversão de `CONST` falha, emite `0` para preservar o avanço da
    /// montagem e continuar coletando diagnósticos.
    fn emit_data_directive(&mut self, directive: DataDirective, node_spans: &NodeSpans) {
        match directive {
            DataDirective::Const { value, node_id } => {
                let directive_span = node_spans.span_of(node_id);
                let value_span = node_spans.span_of(value.node_id);
                let value = self
                    .parse_word_literal(value.literal, NumberContext::ConstValue, value_span)
                    .unwrap_or(0);
                self.push_resolved_word(value, directive_span);
            }
            DataDirective::Space { amount, node_id } => {
                let directive_span = node_spans.span_of(node_id);
                let amount_span = amount
                    .as_ref()
                    .map(|amount| node_spans.span_of(amount.node_id))
                    .unwrap_or(directive_span);
                if let Some(amount) = self.parse_space_amount(amount, amount_span) {
                    for _ in 0..amount {
                        self.push_resolved_word(0, directive_span);
                    }
                }
            }
        }
    }

    /// Define um rótulo no endereço corrente.
    ///
    /// Contrato:
    /// - rótulo novo cria uma entrada [`SymbolEntry::Defined`];
    /// - rótulo com usos pendentes resolve e remenda todos os [`PatchSite`] no
    ///   `.obj`, preservando a lista embutida no `.pen`;
    /// - rótulo já definido gera diagnóstico de duplicidade e preserva a
    ///   primeira definição.
    ///
    /// # Parâmetros
    /// - `label`: definição de rótulo produzida pelo parser;
    /// - `node_spans`: tabela lateral usada para localizar o rótulo.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::SymbolAlreadyDefined`] quando o mesmo
    /// símbolo já possuía definição, e [`AssemblyErrorKind::AddressOverflow`]
    /// se o contador corrente não cabe em [`Word`].
    fn define_symbol(&mut self, label: LabelDef, node_spans: &NodeSpans) {
        let span = node_spans.span_of(label.node_id);
        let address = self.current_address(span).unwrap_or(0);

        match self.symbols.entries.remove(&label.name) {
            Some(SymbolEntry::Defined {
                defined_at,
                address,
            }) => {
                self.errors.push(AssemblyError {
                    kind: AssemblyErrorKind::SymbolAlreadyDefined {
                        symbol: label.name,
                        first_defined_at: defined_at,
                    },
                    span,
                });
                self.symbols.entries.insert(
                    label.name,
                    SymbolEntry::Defined {
                        address,
                        defined_at,
                    },
                );
            }
            Some(SymbolEntry::Pending { uses, .. }) => {
                for patch in uses {
                    self.patch_object_word(address, patch);
                }

                self.symbols.entries.insert(
                    label.name,
                    SymbolEntry::Defined {
                        address,
                        defined_at: span,
                    },
                );
            }
            None => {
                self.symbols.entries.insert(
                    label.name,
                    SymbolEntry::Defined {
                        address,
                        defined_at: span,
                    },
                );
            }
        }
    }

    /// Remenda no `.obj` uma palavra provisória emitida por referência futura.
    ///
    /// A função aplica o deslocamento armazenado no [`PatchSite`] ao endereço
    /// base do símbolo e substitui a palavra no índice emitido anteriormente.
    /// O `.pen` não é alterado por esta função.
    ///
    /// # Parâmetros
    /// - `base_address`: endereço do símbolo recém-definido;
    /// - `patch`: ponto de uso que precisa ser atualizado.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::AddressOverflow`] quando o endereço
    /// deslocado não cabe em [`Word`].
    fn patch_object_word(&mut self, base_address: Word, patch: PatchSite) {
        let Some(resolved) = Self::checked_address(base_address, patch.offset) else {
            self.errors.push(AssemblyError {
                kind: AssemblyErrorKind::AddressOverflow {
                    address: Self::saturating_address_for_diagnostic(base_address, patch.offset),
                },
                span: patch.span,
            });
            return;
        };

        if let Some(word) = self.artifacts.object.words.get_mut(patch.output_index) {
            *word = resolved;
        }
    }

    /// Emite diagnósticos para símbolos que permaneceram pendentes.
    ///
    /// Esta função roda ao final da passagem. Toda entrada ainda em
    /// [`SymbolEntry::Pending`] representa um símbolo usado, mas nunca definido
    /// em `TEXT` nem em `DATA`.
    ///
    /// # Erros
    /// Para cada uso pendente, acumula
    /// [`AssemblyErrorKind::UndefinedSymbol`] apontando para o span do uso.
    fn report_undefined_symbols(&mut self) {
        for (symbol, entry) in &self.symbols.entries {
            if let SymbolEntry::Pending { uses, .. } = entry {
                for patch in uses {
                    self.errors.push(AssemblyError {
                        kind: AssemblyErrorKind::UndefinedSymbol { symbol: *symbol },
                        span: patch.span,
                    });
                }
            }
        }
    }

    /// Adiciona uma palavra resolvida ao `.obj` e ao `.pen`.
    ///
    /// # Parâmetros
    /// - `word`: palavra a ser emitida;
    /// - `span`: região associada à emissão, usada se o endereço corrente
    ///   estourar.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::AddressOverflow`] quando o contador de
    /// posição atual não cabe em [`Word`]. A palavra ainda é emitida nos dois
    /// artefatos para manter a passagem alinhada.
    fn push_resolved_word(&mut self, word: Word, span: Span) {
        self.ensure_current_address(span);
        self.artifacts.object.words.push(word);
        self.artifacts.pending.words.push(word);
        self.location_counter += 1;
    }

    /// Adiciona uma referência futura ao `.obj` e ao `.pen`.
    ///
    /// O `.obj` recebe `0` como placeholder para remendo posterior. O `.pen`
    /// recebe o elo anterior da lista de pendências do símbolo, ou `0` quando
    /// este é o primeiro uso pendente.
    ///
    /// # Parâmetros
    /// - `previous_head`: posição anterior da lista de pendências do símbolo;
    /// - `span`: região associada ao uso do símbolo.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::AddressOverflow`] se o elo anterior não
    /// couber em [`Word`].
    fn push_unresolved_address(&mut self, previous_head: Option<usize>, span: Span) {
        let pending_link = self.pending_link_word(previous_head, span);
        self.ensure_current_address(span);
        self.artifacts.object.words.push(0);
        self.artifacts.pending.words.push(pending_link);
        self.location_counter += 1;
    }

    /// Converte uma posição de pendência para a palavra gravada no `.pen`.
    ///
    /// # Parâmetros
    /// - `previous_head`: índice anterior da lista embutida no código;
    /// - `span`: região associada ao uso atual, usada em diagnóstico.
    ///
    /// # Retorno
    /// - `0` quando não havia uso pendente anterior;
    /// - o índice anterior convertido para [`Word`] quando possível;
    /// - `0` em caso de estouro, depois de registrar diagnóstico.
    fn pending_link_word(&mut self, previous_head: Option<usize>, span: Span) -> Word {
        let Some(previous_head) = previous_head else {
            return 0;
        };

        match Word::try_from(previous_head) {
            Ok(word) => word,
            Err(_) => {
                self.errors.push(AssemblyError {
                    kind: AssemblyErrorKind::AddressOverflow {
                        address: previous_head,
                    },
                    span,
                });
                0
            }
        }
    }

    /// Converte o contador de posição atual para [`Word`].
    ///
    /// # Parâmetros
    /// - `span`: região associada ao ponto em que o endereço foi consultado.
    ///
    /// # Retorno
    /// - `Some(address)` quando `self.location_counter` cabe na palavra da ISA;
    /// - `None` quando o contador ultrapassa o limite de [`Word`].
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::AddressOverflow`] quando a conversão falha.
    fn current_address(&mut self, span: Span) -> Option<Word> {
        match Word::try_from(self.location_counter) {
            Ok(address) => Some(address),
            Err(_) => {
                self.errors.push(AssemblyError {
                    kind: AssemblyErrorKind::AddressOverflow {
                        address: self.location_counter,
                    },
                    span,
                });
                None
            }
        }
    }

    /// Valida o endereço corrente apenas pelo efeito diagnóstico.
    ///
    /// Este helper é usado antes de emissões que devem prosseguir mesmo quando
    /// o contador já estourou. O retorno de [`Self::current_address`] é
    /// descartado de propósito; o contrato aqui é registrar o erro e manter a
    /// passagem andando.
    ///
    /// # Parâmetros
    /// - `span`: região associada à emissão que consultou o endereço.
    fn ensure_current_address(&mut self, span: Span) {
        let _ = self.current_address(span);
    }

    /// Calcula `base_address + offset` com checagem de faixa.
    ///
    /// # Parâmetros
    /// - `base_address`: endereço do símbolo definido;
    /// - `offset`: deslocamento aplicado pelo operando.
    ///
    /// # Retorno
    /// - `Some(address)` quando o resultado cabe em [`Word`];
    /// - `None` quando o resultado fica negativo ou ultrapassa o limite da
    ///   palavra da arquitetura.
    fn checked_address(base_address: Word, offset: Offset) -> Option<Word> {
        let address = i64::from(base_address) + i64::from(offset);
        if (0..=i64::from(Word::MAX)).contains(&address) {
            Some(address as Word)
        } else {
            None
        }
    }

    /// Calcula uma representação defensiva de endereço para diagnóstico.
    ///
    /// Quando o endereço real pode ser negativo, o tipo do erro ainda espera
    /// `usize`. Este helper preserva o valor positivo quando possível e satura
    /// negativos para `0`, evitando outro erro durante a construção do
    /// diagnóstico.
    ///
    /// # Parâmetros
    /// - `base_address`: endereço do símbolo definido;
    /// - `offset`: deslocamento aplicado pelo operando.
    ///
    /// # Retorno
    /// - endereço como `usize` quando a soma é positiva;
    /// - `0` quando a soma é negativa.
    fn saturating_address_for_diagnostic(base_address: Word, offset: Offset) -> usize {
        let address = i64::from(base_address) + i64::from(offset);
        usize::try_from(address).unwrap_or(0)
    }

    /// Converte o literal de deslocamento de um operando de endereço.
    ///
    /// # Parâmetros
    /// - `literal`: literal numérico preservado pelo parser;
    /// - `span`: região do operando completo usada em diagnósticos.
    ///
    /// # Retorno
    /// - `Some(offset)` quando o literal é numericamente válido;
    /// - `None` quando a conversão falha.
    ///
    /// # Erros
    /// Acumula erro numérico com contexto
    /// [`NumberContext::AddressOffset`].
    fn parse_address_offset(&mut self, literal: NumericLiteral, span: Span) -> Option<Offset> {
        match literal.sign {
            Some(sign) => self
                .parse_signed_number(sign, literal.symbol, NumberContext::AddressOffset, span)
                .map(i32::from),
            None => self
                .parse_unsigned_number(literal.symbol, NumberContext::AddressOffset, span)
                .map(i32::from),
        }
    }

    /// Converte um literal numérico para a palavra emitida pela arquitetura.
    ///
    /// Literais assinados são convertidos como [`crate::assembler::SignedWord`]
    /// e depois reinterpretados como [`Word`], preservando a representação de
    /// complemento de dois usada pela máquina alvo.
    ///
    /// # Parâmetros
    /// - `literal`: literal numérico preservado pelo parser;
    /// - `context`: posição semântica do número, usada no diagnóstico;
    /// - `span`: região associada ao literal ou à diretiva.
    ///
    /// # Retorno
    /// - `Some(word)` quando o número é válido para a arquitetura;
    /// - `None` quando a conversão falha.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::InvalidNumber`] ou
    /// [`AssemblyErrorKind::NumberOverflow`].
    fn parse_word_literal(
        &mut self,
        literal: NumericLiteral,
        context: NumberContext,
        span: Span,
    ) -> Option<Word> {
        match literal.sign {
            Some(sign) => self
                .parse_signed_number(sign, literal.symbol, context, span)
                .map(|value| value as Word),
            None => self.parse_unsigned_number(literal.symbol, context, span),
        }
    }

    /// Converte a quantidade opcional da diretiva `SPACE`.
    ///
    /// Forma canônica:
    ///
    /// ```ignore
    /// BUFFER SPACE
    /// TABLE SPACE 10
    /// ```
    ///
    /// # Parâmetros
    /// - `amount`: literal opcional após `SPACE`;
    /// - `span`: região da diretiva usada em diagnósticos.
    ///
    /// # Retorno
    /// - `Some(1)` quando a diretiva não informa quantidade;
    /// - `Some(amount)` quando a quantidade é positiva;
    /// - `None` quando o literal é inválido, estoura ou representa quantidade
    ///   menor ou igual a zero.
    ///
    /// # Erros
    /// Acumula erro numérico com contexto [`NumberContext::SpaceAmount`] ou
    /// [`AssemblyErrorKind::InvalidSpaceAmount`].
    fn parse_space_amount(&mut self, amount: Option<NumberLiteral>, span: Span) -> Option<usize> {
        let Some(amount) = amount else {
            return Some(1);
        };

        let symbol = amount.literal.symbol;
        let parsed = match amount.literal.sign {
            Some(sign) => self
                .parse_signed_number(sign, symbol, NumberContext::SpaceAmount, span)
                .map(i32::from),
            None => self
                .parse_unsigned_number(symbol, NumberContext::SpaceAmount, span)
                .map(i32::from),
        }?;

        if parsed <= 0 {
            self.errors.push(AssemblyError {
                kind: AssemblyErrorKind::InvalidSpaceAmount { value: symbol },
                span,
            });
            None
        } else {
            Some(parsed as usize)
        }
    }

    /// Converte um símbolo de número sem sinal para [`Word`].
    ///
    /// # Parâmetros
    /// - `symbol`: símbolo internado do lexema numérico;
    /// - `context`: posição semântica do número, usada no diagnóstico;
    /// - `span`: região associada ao uso do número.
    ///
    /// # Retorno
    /// - `Some(word)` quando o lexema é válido e cabe em [`Word`];
    /// - `None` quando a conversão falha.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::InvalidNumber`] ou
    /// [`AssemblyErrorKind::NumberOverflow`].
    fn parse_unsigned_number(
        &mut self,
        symbol: Symbol,
        context: NumberContext,
        span: Span,
    ) -> Option<Word> {
        match numeric_literals::parse_unsigned_symbol(symbol, self.interner) {
            Ok(value) => Some(value),
            Err(err) => {
                self.push_number_error(err, symbol, context, span);
                None
            }
        }
    }

    /// Converte um símbolo de número com sinal explícito para `i16`.
    ///
    /// # Parâmetros
    /// - `sign`: sinal separado pelo lexer/parser;
    /// - `symbol`: símbolo internado do lexema numérico sem o sinal;
    /// - `context`: posição semântica do número, usada no diagnóstico;
    /// - `span`: região associada ao uso do número.
    ///
    /// # Retorno
    /// - `Some(value)` quando o lexema assinado é válido e cabe em `i16`;
    /// - `None` quando a conversão falha.
    ///
    /// # Erros
    /// Acumula [`AssemblyErrorKind::InvalidNumber`] ou
    /// [`AssemblyErrorKind::NumberOverflow`].
    fn parse_signed_number(
        &mut self,
        sign: numeric_literals::NumberSign,
        symbol: Symbol,
        context: NumberContext,
        span: Span,
    ) -> Option<i16> {
        match numeric_literals::parse_signed_symbol(sign, symbol, self.interner) {
            Ok(value) => Some(value),
            Err(err) => {
                self.push_number_error(err, symbol, context, span);
                None
            }
        }
    }

    /// Converte um erro técnico de parsing numérico em erro de montagem.
    ///
    /// Este helper preserva o símbolo original e o contexto semântico para que
    /// a camada de diagnóstico consiga distinguir, por exemplo, falha em
    /// `CONST`, em `SPACE` ou em deslocamento de endereço.
    ///
    /// # Parâmetros
    /// - `err`: erro retornado pelo parser numérico da linguagem;
    /// - `value`: símbolo internado do lexema numérico;
    /// - `context`: posição semântica em que a conversão foi tentada;
    /// - `span`: região associada ao uso do número.
    ///
    /// # Efeitos colaterais
    /// Adiciona [`AssemblyError`] em `self.errors`.
    fn push_number_error(
        &mut self,
        err: NumberParseError,
        value: Symbol,
        context: NumberContext,
        span: Span,
    ) {
        let kind = match err {
            NumberParseError::InvalidNumber => AssemblyErrorKind::InvalidNumber { value, context },
            NumberParseError::Overflow => AssemblyErrorKind::NumberOverflow { value, context },
        };

        self.errors.push(AssemblyError { kind, span });
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
