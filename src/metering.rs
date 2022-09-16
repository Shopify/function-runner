use anyhow::{anyhow, Result};
use parity_wasm::{
    self,
    elements::{
        External, FunctionType, ImportEntry, ImportSection, Instruction, Internal, Module, Section,
        Serialize, Type, TypeSection, ValueType,
    },
};

// TODO: Make me better
fn cost_function(instr: &Instruction) -> i32 {
    match instr {
        Instruction::Nop => 0,
        Instruction::Block(_) => 0,
        Instruction::Loop(_) => 0,
        Instruction::If(_) => 0,
        Instruction::Else => 0,
        Instruction::End => 0,
        Instruction::Drop => 0,
        _ => 1,
    }
}

// This function does two things:
// 1. It injects `(import "env" "consume_gas" (func (param i32)))`
//    into the import section
// 2. It prependes a call to that imported function to each other instruction,
//    invoking it with the cost of the following instruction, as determined
//    by `cost_function`.
//
// TODO: Allow to use a custom cost function.
pub fn meterize(binary: &[u8]) -> Result<Vec<u8>> {
    let mut module: Module = parity_wasm::deserialize_buffer(binary)
        .map_err(|err| anyhow!("Could not deserialize wasm module: {}", err))?;
    inject_metering_func(&mut module)?;
    inject_metering_instructions(&mut module, cost_function)?;

    let mut output: Vec<u8> = Vec::with_capacity(binary.len());
    module.serialize(&mut output)?;
    Ok(output)
}

fn inject_metering_func(module: &mut Module) -> Result<()> {
    // Add a new type for the imported metering function.
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

    // Prepend the metering function to imports. This is easier as it
    // implicitly gives the metering function ID 0 and *every* other
    // function reference ID needs get +1'd.
    if module.import_section().is_none() {
        module.insert_section(Section::Import(ImportSection::with_entries(vec![])))?;
    }
    let imports = module.import_section_mut().unwrap().entries_mut();
    *imports = [ImportEntry::new(
        "env".to_string(),
        "consume_gas".to_string(),
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
    // TODO: Handle elem section and other function references

    Ok(())
}

fn inject_metering_instructions<T>(module: &mut Module, meter_func: T) -> Result<()>
where
    T: Fn(&Instruction) -> i32,
{
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
                    Instruction::I32Const(meter_func(item)),
                    Instruction::Call(0),
                    item.clone(),
                ]
                .into_iter(),
            })
            .collect();
    }
    Ok(())
}
