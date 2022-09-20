use std::collections::HashMap;

use anyhow::{anyhow, Result};
use parity_wasm::{
    self,
    elements::{
        External, FunctionType, ImportEntry, ImportSection, Instruction, Internal, Module, Section,
        Serialize, Type, TypeSection, ValueType,
    },
};

pub struct InstrCounter {
    counters: HashMap<i32, u64>,
    instr_map: HashMap<String, i32>,
}

impl InstrCounter {
    pub fn new() -> InstrCounter {
        InstrCounter {
            counters: HashMap::new(),
            instr_map: HashMap::new(),
        }
    }

    pub fn inc(&mut self, instr: i32) {
        if let Some(ctr) = self.counters.get_mut(&instr) {
            *ctr += 1;
        } else {
            self.counters.insert(instr, 1);
        }
    }

    pub fn id_for_instruction(&mut self, instr: &Instruction) -> i32 {
        // To get a unique identifier for a given instruction
        // stringify it, cut off everything after the first space
        // and use the first bit as a unique identifier :shrug:
        // FIXME: This feels brittle.
        let instr = instr.to_string();
        let first_space = instr.chars().position(|c| c == ' ').unwrap_or(instr.len());
        let instr = instr[0..first_space].to_string();
        if let Some(id) = self.instr_map.get(&instr) {
            *id
        } else {
            let id = self.instr_map.len() as i32;
            self.instr_map.insert(instr.clone(), id);
            id
        }
    }

    pub fn instruction_for_id(&self, id: i32) -> Option<String> {
        self.instr_map
            .iter()
            .find(|(_key, value)| **value == id)
            .map(|(key, _value)| key.clone())
    }

    pub fn total_count(&self) -> impl Iterator<Item = (String, u64)> + '_ {
        self.counters.iter().map(|(id, count)| {
            (
                self.instruction_for_id(*id)
                    .unwrap_or_else(|| "<unknown instruciton>".to_string()),
                *count,
            )
        })
    }

    // This function does two things:
    // 1. It injects `(import "instruction_counter" "inc" (func (param i32)))`
    //    into the import section
    // 2. It prependes a call to that imported function to each other instruction,
    //    invoking it with the cost of the following instruction, as determined
    //    by `cost_function`.
    //
    // TODO: Allow to use a custom cost function.
    pub fn counterize(&mut self, binary: &[u8]) -> Result<Vec<u8>> {
        let mut module: Module = parity_wasm::deserialize_buffer(binary)
            .map_err(|err| anyhow!("Could not deserialize wasm module: {}", err))?;
        self.inject_counting_func(&mut module)?;
        self.inject_counting_instructions(&mut module)?;

        let mut output: Vec<u8> = Vec::with_capacity(binary.len());
        module.serialize(&mut output)?;
        Ok(output)
    }

    fn inject_counting_func(&self, module: &mut Module) -> Result<()> {
        // Add a new type for the imported counting function.
        // TODO: Dedupe types. For now, de-duping can be taken care of by wasm-opt.
        if module.type_section().is_none() {
            module.insert_section(Section::Type(TypeSection::with_types(vec![])))?;
        }
        let types = module.type_section_mut().unwrap().types_mut();
        types.push(Type::Function(FunctionType::new(
            vec![ValueType::I32],
            vec![],
        )));
        let type_id = types.len() - 1;

        // Prepend the counting function to imports. This is easier as it
        // implicitly gives the counting function ID 0 and *every* other
        // function reference ID needs get +1'd.
        if module.import_section().is_none() {
            module.insert_section(Section::Import(ImportSection::with_entries(vec![])))?;
        }
        let imports = module.import_section_mut().unwrap().entries_mut();
        *imports = [ImportEntry::new(
            "instruction_counter".to_string(),
            "inc".to_string(),
            External::Function(type_id as u32),
        )]
        .into_iter()
        .chain(imports.iter().cloned())
        .collect();

        // For each function export entry, add 1 to the ID
        module.export_section_mut().map(|export_section| {
            for entry in export_section.entries_mut().iter_mut() {
                match entry.internal_mut() {
                    Internal::Function(v) => *v += 1,
                    _ => {}
                };
            }
        });

        // For each `call` instruction, add 1 to the ID
        module.code_section_mut().map(|code_section| {
            for body in code_section.bodies_mut() {
                for instr in body.code_mut().elements_mut() {
                    match instr {
                        Instruction::Call(v) => *v += 1,
                        _ => {}
                    };
                }
            }
        });

        module.elements_section_mut().map(|element_section| {
            for entry in element_section.entries_mut() {
                for member in entry.members_mut() {
                    *member += 1
                }
            }
        });
        // TODO: Handle other function references?

        Ok(())
    }

    fn inject_counting_instructions(&mut self, module: &mut Module) -> Result<()> {
        let code_section = match module.code_section_mut() {
            None => return Ok(()),
            Some(f) => f,
        };
        for body in code_section.bodies_mut() {
            let instr = body.code_mut().elements_mut();

            *instr = instr
                .iter()
                .flat_map(|item| match item {
                    // TODO: Do any instruction need special handling?
                    _ => [
                        Instruction::I32Const(self.id_for_instruction(item)),
                        Instruction::Call(0),
                        item.clone(),
                    ]
                    .into_iter(),
                })
                .collect();
        }
        Ok(())
    }
}
