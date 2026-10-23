//! Source diagnostics after exact signature replay, before unchanged scalar ABI issuance.
use super::{Failure, raw, reject, source::Originals};
use zryna_diagnostics::Diagnostic;
use zryna_source::SourceMap;

#[cfg(test)]
mod tests;

pub(super) fn check(
    program: &raw::Program,
    originals: &Originals<'_>,
    sources: &SourceMap,
    entry: u32,
) -> Result<(), Failure> {
    let unit = originals
        .units
        .get(entry as usize)
        .filter(|unit| unit.id == entry)
        .ok_or_else(|| reject("unknown source ABI entry module"))?;
    for (index, original) in unit.functions.iter().enumerate() {
        // Template visibility and imported definitions are not executable entry exports.
        if original.export_span.is_none() || original.type_parameters.is_some() {
            continue;
        }
        let index = u32::try_from(index).map_err(|_| Failure::InternalFailure)?;
        let mut key = [0u8; 9];
        key[0] = 0x41;
        key[1..5].copy_from_slice(&entry.to_le_bytes());
        key[5..9].copy_from_slice(&index.to_le_bytes());
        let index = program
            .functions
            .binary_search_by(|function| function.key.as_slice().cmp(&key))
            .map_err(|_| reject("source ABI export has no exact original root"))?;
        let function = &program.functions[index];
        // Only call after source substitution: canonical stored IDs 0/1 are bool/i32.
        let scalar = |ty| matches!(ty, raw::Type::Stored(0 | 1));
        if !function.parameters.iter().copied().all(scalar) || !scalar(function.result) {
            let span =
                sources.verify_span(original.name.span).map_err(|_| Failure::InternalFailure)?;
            return Err(Failure::Diagnostics(vec![Diagnostic::error_at(
                "ZRYNA-M7005",
                span,
                "exported function signature cannot cross the selected scalar ABI",
                "keep generic and aggregate values internal; export a scalar wrapper",
            )]));
        }
    }
    Ok(())
}
