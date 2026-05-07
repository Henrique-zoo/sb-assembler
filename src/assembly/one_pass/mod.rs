use crate::{
    assembly::one_pass::types::{AssemblyArtifacts, ObjectProgram, OnePassAssembler, PendingProgram, SymbolEntry},
    errors::{AssemblyError, AssemblyErrorKind},
    parser::{ir::LabelDef, types::{DataLine, ParsedProgram}},
};

mod types;

impl<'a> OnePassAssembler<'a> {
    pub(crate) fn assemble(
        mut self,
        program: ParsedProgram,
    ) -> Result<AssemblyArtifacts, Vec<AssemblyError>> {
        let pending = PendingProgram {
            words: Vec::new()
        };
        let object = ObjectProgram {
            words: Vec::new(),
        };
        let object = self.assemble_data(program.data, object)?;


        Ok(AssemblyArtifacts {
            object,
            pending
        })
    }

    fn assemble_data(
        &mut self,
        data: Vec<DataLine>,
        object: ObjectProgram,
    ) -> Result<ObjectProgram, Vec<AssemblyError>> {
        data.into_iter()
            .fold(Ok(object), |acc, parsed_line| {
                let mut object = acc?;

                let LabelDef{ name: label, span } = parsed_line
                    .label
                    .ok_or_else(|| {
                        vec![AssemblyError {
                            kind: AssemblyErrorKind::NoRotuleInDataLine,
                            span: parsed_line.span,
                        }]
                    })?;

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
                                object.words[u.output_index] = self.location_counter as u16;
                            }
                        }
                    }

                    self.symbols.entries.insert(
                        label,
                        SymbolEntry::Defined {
                            address: self.location_counter as u16,
                            defined_at: span,
                        },
                    );
                } else {
                    self.symbols.entries.insert(
                        label,
                        SymbolEntry::Defined {
                            address: self.location_counter as u16,
                            defined_at: span,
                        },
                    );
                }

                Ok(object)
            })
    }
}
