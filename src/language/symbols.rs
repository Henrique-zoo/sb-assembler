use std::collections::{HashMap, HashSet};

use crate::{
    interner::{Interner, Symbol},
    language::instructions::Mnemonic,
};

/// Símbolos internados das diretivas próprias do pré-processador.
#[derive(Debug)]
pub(crate) struct PreprocessorSymbols {
    /// Keyword `SECTION`.
    pub section: Symbol,
    /// Keyword `MACRO`.
    pub macro_: Symbol,
    /// Keyword `ENDMACRO`.
    pub endmacro: Symbol,
    /// Keyword `EQU`.
    pub equ: Symbol,
    /// Keyword `IF`.
    pub if_: Symbol,
}

/// Símbolos internados dos nomes de seção da linguagem.
#[derive(Debug)]
pub(crate) struct SectionSymbols {
    /// Nome de seção `TEXT`.
    pub text: Symbol,
    /// Nome de seção `DATA`.
    pub data: Symbol,
}

/// Símbolos internados das diretivas montáveis da seção `DATA`.
#[derive(Debug)]
pub(crate) struct DataDirectiveSymbols {
    /// Diretiva `CONST`.
    pub const_: Symbol,
    /// Diretiva `SPACE`.
    pub space: Symbol,
}

/// Símbolos internados dos mnemônicos da máquina hipotética.
#[derive(Debug)]
pub(crate) struct InstructionSymbols {
    /// Instrução `ADD`.
    pub add: Symbol,
    /// Instrução `SUB`.
    pub sub: Symbol,
    /// Instrução `MUL`.
    pub mul: Symbol,
    /// Instrução `DIV`.
    pub div: Symbol,
    /// Instrução `JMP`.
    pub jmp: Symbol,
    /// Instrução `JMPN`.
    pub jmpn: Symbol,
    /// Instrução `JMPP`.
    pub jmpp: Symbol,
    /// Instrução `JMPZ`.
    pub jmpz: Symbol,
    /// Instrução `COPY`.
    pub copy: Symbol,
    /// Instrução `LOAD`.
    pub load: Symbol,
    /// Instrução `STORE`.
    pub store: Symbol,
    /// Instrução `INPUT`.
    pub input: Symbol,
    /// Instrução `OUTPUT`.
    pub output: Symbol,
    /// Instrução `STOP`.
    pub stop: Symbol,
}

/// Vocabulário internado da linguagem assembly.
///
/// Esta tabela pertence à camada `language` porque descreve palavras
/// reconhecidas pela linguagem, não o estado de um estágio específico. Lexer,
/// pré-processador, parser e montagem podem receber uma referência para a mesma
/// instância e comparar símbolos sem voltar para strings.
#[derive(Debug)]
pub(crate) struct LanguageSymbols {
    /// Diretivas executadas pelo pré-processador.
    pub preprocessor: PreprocessorSymbols,
    /// Nomes das seções montáveis.
    pub sections: SectionSymbols,
    /// Diretivas aceitas na seção de dados.
    pub data_directives: DataDirectiveSymbols,
    /// Mnemônicos da máquina hipotética.
    pub instructions: InstructionSymbols,
    directives: HashSet<Symbol>,
    instruction_set: HashSet<Symbol>,
    mnemonic_instruction_map: HashMap<Symbol, Mnemonic>,
}

impl LanguageSymbols {
    /// Interna o vocabulário fixo da linguagem e monta as tabelas de consulta.
    pub(crate) fn new(interner: &mut Interner) -> Self {
        let preprocessor = PreprocessorSymbols {
            section: interner.entry("SECTION").or_insert(),
            macro_: interner.entry("MACRO").or_insert(),
            endmacro: interner.entry("ENDMACRO").or_insert(),
            equ: interner.entry("EQU").or_insert(),
            if_: interner.entry("IF").or_insert(),
        };

        let sections = SectionSymbols {
            text: interner.entry("TEXT").or_insert(),
            data: interner.entry("DATA").or_insert(),
        };

        let data_directives = DataDirectiveSymbols {
            const_: interner.entry("CONST").or_insert(),
            space: interner.entry("SPACE").or_insert(),
        };

        let instructions = InstructionSymbols {
            add: interner.entry("ADD").or_insert(),
            sub: interner.entry("SUB").or_insert(),
            mul: interner.entry("MUL").or_insert(),
            div: interner.entry("DIV").or_insert(),
            jmp: interner.entry("JMP").or_insert(),
            jmpn: interner.entry("JMPN").or_insert(),
            jmpp: interner.entry("JMPP").or_insert(),
            jmpz: interner.entry("JMPZ").or_insert(),
            copy: interner.entry("COPY").or_insert(),
            load: interner.entry("LOAD").or_insert(),
            store: interner.entry("STORE").or_insert(),
            input: interner.entry("INPUT").or_insert(),
            output: interner.entry("OUTPUT").or_insert(),
            stop: interner.entry("STOP").or_insert(),
        };

        let instruction_set = [
            instructions.add,
            instructions.sub,
            instructions.mul,
            instructions.div,
            instructions.jmp,
            instructions.jmpn,
            instructions.jmpp,
            instructions.jmpz,
            instructions.copy,
            instructions.load,
            instructions.store,
            instructions.input,
            instructions.output,
            instructions.stop,
        ]
        .into_iter()
        .collect();

        let directives = [
            preprocessor.section,
            preprocessor.macro_,
            preprocessor.endmacro,
            preprocessor.equ,
            preprocessor.if_,
            sections.text,
            sections.data,
            data_directives.const_,
            data_directives.space,
        ]
        .into_iter()
        .collect();

        let mut mnemonic_instruction_map = HashMap::<Symbol, Mnemonic>::new();
        mnemonic_instruction_map.insert(instructions.add, Mnemonic::Add);
        mnemonic_instruction_map.insert(instructions.sub, Mnemonic::Sub);
        mnemonic_instruction_map.insert(instructions.mul, Mnemonic::Mul);
        mnemonic_instruction_map.insert(instructions.div, Mnemonic::Div);
        mnemonic_instruction_map.insert(instructions.jmp, Mnemonic::Jmp);
        mnemonic_instruction_map.insert(instructions.jmpn, Mnemonic::Jmpn);
        mnemonic_instruction_map.insert(instructions.jmpp, Mnemonic::Jmpp);
        mnemonic_instruction_map.insert(instructions.jmpz, Mnemonic::Jmpz);
        mnemonic_instruction_map.insert(instructions.copy, Mnemonic::Copy);
        mnemonic_instruction_map.insert(instructions.load, Mnemonic::Load);
        mnemonic_instruction_map.insert(instructions.store, Mnemonic::Store);
        mnemonic_instruction_map.insert(instructions.input, Mnemonic::Input);
        mnemonic_instruction_map.insert(instructions.output, Mnemonic::Output);
        mnemonic_instruction_map.insert(instructions.stop, Mnemonic::Stop);

        Self {
            preprocessor,
            sections,
            data_directives,
            instructions,
            directives,
            instruction_set,
            mnemonic_instruction_map,
        }
    }

    /// Indica se `sym` é uma diretiva ou nome de seção reservado.
    pub(crate) fn is_directive(&self, sym: Symbol) -> bool {
        self.directives.contains(&sym)
    }

    /// Indica se `sym` é um mnemônico reservado da ISA.
    pub(crate) fn is_instruction(&self, sym: Symbol) -> bool {
        self.instruction_set.contains(&sym)
    }

    /// Indica se `sym` é qualquer palavra reservada da linguagem.
    pub(crate) fn is_reserved(&self, sym: Symbol) -> bool {
        self.is_directive(sym) || self.is_instruction(sym)
    }

    pub(crate) fn get_mnemonic(&self, sym: &Symbol) -> Option<&Mnemonic> {
        self.mnemonic_instruction_map.get(sym)
    }
}
