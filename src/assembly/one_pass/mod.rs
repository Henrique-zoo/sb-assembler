/*use std::{collections::HashMap, ops::Add};

use crate::{
    assembler::Word,
    assembly::one_pass::types::{
        AssemblyArtifacts, ObjectProgram, Offset, OnePassAssembler, PatchSite, PendingProgram,
        SymbolEntry,
    },
    errors::{AssemblyError, AssemblyErrorKind},
    interner::Symbol,
    language::instructions::Mnemonic,
    lexer::Span,
    parser::{
        ir::{AddressOperand, Instruction, LabelDef, SymbolRef},
        types::{DataLine, NodeSpans, ParsedProgram, TextLine},
    },
};

mod types;

impl<'a> OnePassAssembler<'a> {
    pub(crate) fn assemble(
        mut self,
        program: ParsedProgram,
    ) -> Result<AssemblyArtifacts, Vec<AssemblyError>> {
        let ParsedProgram {
            text: _,
            data,
            node_spans,
        } = program;
        let pending = PendingProgram { words: Vec::new() };
        let object = ObjectProgram { words: Vec::new() };
        let object = self.assemble_data(data, object, &node_spans)?;

        Ok(AssemblyArtifacts { object, pending })
    }

    fn assemble_text(
        &mut self,
        text: Vec<TextLine>,
        mut pending: PendingProgram,
        node_spans: &NodeSpans,
    ) -> Result<(PendingProgram, HashMap<Symbol, Vec<PatchSite>>), Vec<AssemblyError>> {
        let mut label_def_locs = HashMap::new();
        let mut errors = Vec::new();
        for parsed_line in text {
            if let Some(LabelDef { name, node_id }) = parsed_line.label {
                let span = node_spans.span_of(node_id);
                if let Some(symbol_entry) = self.symbols.entries.get(&name) {
                    match *symbol_entry {
                        SymbolEntry::Defined {
                            address,
                            defined_at,
                        } => errors.push(AssemblyError {
                            kind: AssemblyErrorKind::SymbolAlreadyDefined {
                                symbol: name,
                                first_defined_at: defined_at,
                            },
                            span: span,
                        }),
                        SymbolEntry::Pending { uses } => {
                            label_def_locs.insert(name, uses);
                        }
                    }
                } else {
                    label_def_locs.insert(name, Vec::new());
                }
            }

            match parsed_line.body {
                Instruction::NoOperand { mnemonic, node_id } => {
                    let span = node_spans.span_of(node_id);
                    self.apply_instruction_effects(
                        mnemonic,
                        span,
                        &mut pending,
                        &mut errors,
                        || {},
                    );
                }
                Instruction::OneOperand {
                    mnemonic,
                    operand,
                    node_id,
                } => {
                    let span = node_spans.span_of(node_id);
                    self.apply_instruction_effects(
                        mnemonic,
                        span,
                        &mut pending,
                        &mut errors,
                        || {
                            match operand {
                                AddressOperand::Direct(symbol_ref) => {
                                    let operand_span = node_spans.span_of(symbol_ref.node_id);
                                    self.apply_operand_effects(
                                        symbol_ref,
                                        &mut pending,
                                        operand_span,
                                    );
                                }
                                AddressOperand::Offset {
                                    base,
                                    offset,
                                    node_id,
                                } => {
                                    let span = node_spans.span_of(node_id);
                                    match offset.literal.sign {
                                        None => (),
                                        Some(_) => (),
                                    }
                                }
                            }
                            self.location_counter += 1;
                        },
                    );
                }
                Instruction::TwoOperands {
                    mnemonic,
                    first,
                    second,
                    node_id,
                } => {}
            }
        }

        OK(())
    }

    fn apply_operand_effects<T>(
        &mut self,
        symbol_ref: SymbolRef,
        pending: &mut PendingProgram,
        span: Span,
        offset: Offset<T>,
    ) where
        T: Add,
    {
        if let Some(symbol_entry) = self.symbols.entries.get_mut(&symbol_ref.name) {
            match symbol_entry {
                SymbolEntry::Defined {
                    address,
                    defined_at,
                } => {
                    pending.words[self.location_counter] = *address;
                }
                SymbolEntry::Pending { uses } => {
                    pending.words[self.location_counter] = 0;
                    uses.push(PatchSite {
                        output_index: self.location_counter,
                        span,
                    });
                }
            }
        } else {
            self.symbols.entries.insert(
                symbol_ref.name,
                SymbolEntry::Pending {
                    uses: vec![PatchSite {
                        output_index: self.location_counter,
                        span,
                    }],
                },
            );
        }
    }

    fn apply_instruction_effects<F>(
        &mut self,
        mnemonic: Mnemonic,
        span: Span,
        pending: &mut PendingProgram,
        errors: &mut Vec<AssemblyError>,
        ops: F,
    ) where
        F: FnMut() -> (),
    {
        let instruction_spec = self
            .isa
            .specs
            .get(&mnemonic)
            .ok_or_else(|| {
                return errors.push(AssemblyError {
                    kind: AssemblyErrorKind::InvalidMnemonic { mnemonic },
                    span,
                });
            })
            .unwrap();

        pending.words[self.location_counter] = instruction_spec.opcode;
        ops;
        self.location_counter += 1;

        Ok(())
    }

    fn assemble_data(
        &mut self,
        data: Vec<DataLine>,
        object: ObjectProgram,
        node_spans: &NodeSpans,
    ) -> Result<ObjectProgram, Vec<AssemblyError>> {
        data.into_iter().fold(Ok(object), |acc, parsed_line| {
            let mut object = acc?;

            let LabelDef {
                name: label,
                node_id,
            } = parsed_line.label.ok_or_else(|| {
                vec![AssemblyError {
                    kind: AssemblyErrorKind::NoRotuleInDataLine,
                    span: node_spans.span_of(parsed_line.node_id),
                }]
            })?;
            let span = node_spans.span_of(node_id);

            if let Some(symbol_entry) = self.symbols.entries.get(&label) {
                match symbol_entry {
                    SymbolEntry::Defined { defined_at, .. } => {
                        return Err(vec![AssemblyError {
                            kind: AssemblyErrorKind::SymbolAlreadyDefined {
                                symbol: label,
                                first_defined_at: *defined_at,
                            },
                            span,
                        }]);
                    }
                    SymbolEntry::Pending { uses } => {
                        for u in uses {
                            object.words[u.output_index] = self.location_counter as Word;
                        }
                    }
                }

                self.symbols.entries.insert(
                    label,
                    SymbolEntry::Defined {
                        address: self.location_counter as Word,
                        defined_at: span,
                    },
                );
            } else {
                self.symbols.entries.insert(
                    label,
                    SymbolEntry::Defined {
                        address: self.location_counter as Word,
                        defined_at: span,
                    },
                );
            }

            Ok(object)
        })
    }
}
*/
