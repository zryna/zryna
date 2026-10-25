//! Actual final-byte carrier audit; ordinary bodies retain their independent semantics checks.
use super::{Diagnostic, FunctionCode, MAX_BYTES, Model, budget, invariant};
use wasmparser::{Encoding, ExternalKind, Parser, Payload, ValType, Validator, WasmFeatures};
use zryna_abi::VerifiedInvocation;
mod carrier;
pub(in crate::generic_execution_v1) fn audit(
    bytes: &[u8],
    m: &Model<'_, '_, '_>,
    code: &[FunctionCode],
    request: &VerifiedInvocation<'_>,
) -> Result<(), Diagnostic> {
    if bytes.len() > MAX_BYTES {
        return Err(budget());
    }
    m.take(bytes.len())?;
    if m.observed() || code.len() != m.total_functions() as usize + 1 {
        return Err(invariant());
    }
    Validator::new_with_features(WasmFeatures::WASM1)
        .validate_all(bytes)
        .map_err(|_| invariant())?;
    let mut stage = 0;
    let mut bodies = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|_| invariant())? {
            Payload::Version { num: 1, encoding: Encoding::Module, .. } | Payload::End(_) => {}
            Payload::TypeSection(reader) => {
                next(&mut stage, 0)?;
                types(reader, code)?;
            }
            Payload::FunctionSection(reader) => {
                next(&mut stage, 1)?;
                super::audit::sections::functions(reader, code.len())?;
            }
            Payload::MemorySection(reader) => {
                next(&mut stage, 2)?;
                super::audit::sections::memory(reader)?;
            }
            Payload::GlobalSection(reader) => {
                next(&mut stage, 3)?;
                super::audit::sections::globals(reader, m)?;
            }
            Payload::ExportSection(reader) => {
                next(&mut stage, 4)?;
                if reader.count() != 1 {
                    return Err(invariant());
                }
                for export in reader {
                    let export = export.map_err(|_| invariant())?;
                    if export.name != crate::generic_execution_v1::invocation::ENTRY
                        || export.kind != ExternalKind::Func
                        || export.index != m.total_functions()
                    {
                        return Err(invariant());
                    }
                }
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
                if bodies < m.total_functions() as usize {
                    super::audit::body::check(bytes, &actual, bodies, m, expected)?;
                } else {
                    let range = actual.range();
                    let start = usize::try_from(range.start).map_err(|_| invariant())?;
                    let end = usize::try_from(range.end).map_err(|_| invariant())?;
                    if bytes.get(start..end) != Some(expected.body.bytes.as_slice())
                        || expected.locals != 2
                    {
                        return Err(invariant());
                    }
                    carrier::check(&actual, m, request)?;
                }
                bodies += 1;
            }
            Payload::DataSection(reader) => {
                next(&mut stage, 6)?;
                super::audit::sections::data(reader, m)?;
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

fn types(
    reader: wasmparser::TypeSectionReader<'_>,
    code: &[FunctionCode],
) -> Result<(), Diagnostic> {
    if reader.count() as usize != code.len() {
        return Err(invariant());
    }
    for (index, ty) in reader.into_iter_err_on_gc_types().enumerate() {
        let ty = ty.map_err(|_| invariant())?;
        let function = &code[index];
        if ty.params().len() != function.parameters as usize
            || ty.results().len() != function.results as usize
            || ty.params().iter().any(|ty| *ty != ValType::I32)
            || ty
                .results()
                .iter()
                .any(|ty| *ty != if function.result64 { ValType::I64 } else { ValType::I32 })
        {
            return Err(invariant());
        }
        if index + 1 == code.len()
            && (function.parameters != 0 || function.results != 1 || !function.result64)
        {
            return Err(invariant());
        }
    }
    Ok(())
}
