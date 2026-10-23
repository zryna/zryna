//! Exact final-byte ownership adapter: defined memory, audited static data and closed code.
use super::{
    Diagnostic, MAX_BYTES, budget, bytes::Bytes, code::FunctionCode, invariant, model::Model,
};
use wasm_encoder::Instruction as I;
mod audit;
pub(super) fn audit(
    bytes: &[u8],
    m: &Model<'_, '_, '_>,
    code: &[FunctionCode],
) -> Result<(), Diagnostic> {
    audit::check(bytes, m, code)
}
pub(super) fn encode(m: &Model<'_, '_, '_>, code: &[FunctionCode]) -> Result<Vec<u8>, Diagnostic> {
    let mut output = Bytes::new();
    output.extend(b"\0asm\x01\0\0\0")?;
    let mut types = Bytes::new();
    types.count(code.len())?;
    for function in code {
        types.extend(&[0x60])?;
        types.u32(function.parameters)?;
        for _ in 0..function.parameters {
            types.extend(&[0x7f])?;
        }
        types.u32(function.results)?;
        for _ in 0..function.results {
            types.extend(&[if function.result64 { 0x7e } else { 0x7f }])?;
        }
    }
    output.section(1, &types)?;
    let mut functions = Bytes::new();
    functions.count(code.len())?;
    for index in 0..code.len() {
        functions.count(index)?;
    }
    output.section(3, &functions)?;
    let mut memory = Bytes::new();
    memory.u32(1)?;
    memory.u32(1)?;
    memory.u32(256)?;
    memory.u32(256)?;
    output.section(5, &memory)?;
    let mut globals = Bytes::new();
    globals.u32(10)?;
    for index in 0..10 {
        globals.extend(&[0x7f, 1])?;
        globals.op(&I::I32Const(if index == 0 {
            crate::generic_execution_v1::checked_u32(m.memory.claims().heap_start)?.cast_signed()
        } else {
            0
        }))?;
        globals.op(&I::End)?;
    }
    output.section(6, &globals)?;
    let mut exports = Bytes::new();
    exports.count(m.program.export_functions().len() + usize::from(m.observed()))?;
    for (index, export) in m.program.scalar_abi().exports().enumerate() {
        exports.name(export.webassembly_name().as_str())?;
        exports.extend(&[0])?;
        exports.u32(m.wrapper_base + crate::generic_execution_v1::checked_u32(index)?)?;
    }
    if m.observed() {
        exports.name("$zryna$observation")?;
        exports.extend(&[0])?;
        exports.u32(m.observer)?;
    }
    output.section(7, &exports)?;
    let mut bodies = Bytes::new();
    bodies.count(code.len())?;
    for function in code {
        bodies.count(function.body.bytes.len())?;
        bodies.extend(&function.body.bytes)?;
    }
    output.section(10, &bodies)?;
    let mut data = Bytes::new();
    data.u32(1)?;
    data.u32(0)?;
    data.op(&I::I32Const(0))?;
    data.op(&I::End)?;
    data.u32(m.memory.claims().heap_start)?;
    let final_size = output
        .bytes
        .len()
        .checked_add(data.bytes.len())
        .and_then(|n| n.checked_add(m.memory.claims().heap_start as usize))
        .and_then(|n| n.checked_add(6))
        .filter(|n| *n <= MAX_BYTES)
        .ok_or_else(budget)?;
    let _ = final_size;
    let mut cursor = 0usize;
    for literal in &m.memory.claims().literals {
        zeros(&mut data, literal.offset as usize - cursor)?;
        data.extend(&literal.bytes)?;
        cursor = literal.offset as usize + literal.bytes.len();
    }
    zeros(&mut data, m.memory.claims().heap_start as usize - cursor)?;
    output.section(11, &data)?;
    Ok(output.bytes)
}
fn zeros(data: &mut Bytes, mut count: usize) -> Result<(), Diagnostic> {
    let zeros = [0u8; 4096];
    while count != 0 {
        let chunk = count.min(zeros.len());
        data.extend(&zeros[..chunk])?;
        count -= chunk;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn independent_audit(bytes: &[u8], m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    audit::independent(bytes, m)
}
