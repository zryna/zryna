//! Parses actual final bytes; checks every capability, address partition and exact recipe body.
use super::{Diagnostic, FunctionCode, MAX_BYTES, Model, budget, invariant};
use wasmparser::{Encoding, Parser, Payload, Validator, WasmFeatures};
pub(super) mod body;
pub(super) mod sections;
mod semantics;
pub(super) fn check(
    bytes: &[u8],
    m: &Model<'_, '_, '_>,
    code: &[FunctionCode],
) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_BYTES {
        return Err(budget());
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(bytes)
        .map_err(|_| invariant())?;
    let mut stage = 0u8;
    let mut bodies = 0usize;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|_| invariant())? {
            Payload::Version { num: 1, encoding: Encoding::Module, .. } | Payload::End(_) => {}
            Payload::TypeSection(reader) => {
                next(&mut stage, 0)?;
                sections::types(reader, m, code.len())?;
            }
            Payload::FunctionSection(reader) => {
                next(&mut stage, 1)?;
                sections::functions(reader, code.len())?;
            }
            Payload::MemorySection(reader) => {
                next(&mut stage, 2)?;
                sections::memory(reader)?;
            }
            Payload::GlobalSection(reader) => {
                next(&mut stage, 3)?;
                sections::globals(reader, m)?;
            }
            Payload::ExportSection(reader) => {
                next(&mut stage, 4)?;
                sections::exports(reader, m)?;
            }
            Payload::CodeSectionStart { count, .. } => {
                next(&mut stage, 5)?;
                if count as usize != code.len() {
                    return Err(invariant());
                }
            }
            Payload::CodeSectionEntry(actual) => {
                if stage != 6 {
                    return Err(invariant());
                }
                let expected = code.get(bodies).ok_or_else(invariant)?;
                body::check(bytes, &actual, bodies, m, expected)?;
                bodies += 1;
            }
            Payload::DataSection(reader) => {
                next(&mut stage, 6)?;
                sections::data(reader, m)?;
            }
            _ => return Err(invariant()),
        }
    }
    if stage != 7 || bodies != code.len() {
        return Err(invariant());
    }
    Ok(())
}
fn next(stage: &mut u8, expected: u8) -> Result<(), Diagnostic> {
    if *stage != expected {
        return Err(invariant());
    }
    *stage += 1;
    Ok(())
}

#[cfg(test)]
pub(super) fn independent(bytes: &[u8], m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(bytes)
        .map_err(|_| invariant())?;
    let mut index = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CodeSectionEntry(body) = payload.map_err(|_| invariant())? {
            semantics::check(&body, index, m)?;
            index += 1;
        }
    }
    if index != m.total_functions() as usize {
        return Err(invariant());
    }
    Ok(())
}
