//! Exact original scalar export selection; this phase cannot issue an execution seal.

use super::{Failure, budget::charge, raw, reject, reserve};
use zryna_diagnostics::Diagnostic;
use zryna_source::{FileId, SourceMap};
use zryna_syntax::v5::{
    RawFunctionSyntax, RawSourceUnit, RawTypeSyntaxKind, VerifiedProjectSyntaxV5,
};

pub(super) fn derive(
    program: &raw::Program,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    spent: &mut usize,
) -> Result<(zryna_abi::VerifiedScalarAbiModule, Vec<usize>), Failure> {
    let mut used = *spent;
    if !syntax.is_bound_to(sources)
        || sources.verify_file_id(entry.index()).ok() != Some(entry)
        || program.entry_module != entry.index()
    {
        return Err(reject("foreign full-profile scalar export authority"));
    }
    let unit = syntax
        .files()
        .get(entry.index() as usize)
        .filter(|unit| unit.id == entry.index())
        .ok_or_else(|| reject("unknown original scalar export module"))?;
    charge(&mut used, program.functions.len())?;
    let mut selected = reserve(program.functions.len())?;
    selected.resize(program.functions.len(), false);
    // Execution-owned raw ABI rows and the retained function-index selection each
    // reserve one slot per original. Both capacities are prepaid before either reserve.
    let retained = unit.functions.len().checked_mul(2).ok_or(Failure::InternalFailure)?;
    charge(&mut used, retained)?;
    let mut exports = reserve(unit.functions.len())?;
    let mut indices = reserve(unit.functions.len())?;
    for (ordinal, original) in unit.functions.iter().enumerate() {
        if original.export_span.is_none() || original.type_parameters.is_some() {
            continue;
        }
        let ordinal = u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?;
        let mut key = [0; 9];
        key[0] = 0x41;
        key[1..5].copy_from_slice(&entry.index().to_le_bytes());
        key[5..9].copy_from_slice(&ordinal.to_le_bytes());
        let index = find(program, &key, &mut used)?;
        let function = &program.functions[index];
        charge(&mut used, original.name.text.len())?;
        if function.original_span != original.span
            || function.executable_export.as_deref() != Some(original.name.text.as_str())
        {
            return Err(reject("scalar export is not its exact original root/name"));
        }
        if function.parameters.len() != original.parameters.len() {
            return Err(reject("scalar export parameter count differs from its original"));
        }
        charge(&mut used, original.parameters.len())?;
        let mut parameters = reserve(original.parameters.len())?;
        for (parameter, source) in function.parameters.iter().zip(&original.parameters) {
            let expected = source_scalar(unit, source.type_syntax, original, sources, &mut used)?;
            let raw::Parameter::Value(definition) = parameter else {
                return Err(reject("scalar export parameter cannot carry a loan"));
            };
            if claimed_scalar(program, definition.ty, &mut used)? != expected {
                return Err(reject("scalar export parameter differs from its original type"));
            }
            parameters.push(expected);
        }
        let result = source_scalar(unit, original.result_type, original, sources, &mut used)?;
        if claimed_scalar(program, function.result_type, &mut used)? != result {
            return Err(reject("scalar export result differs from its original type"));
        }
        charge(&mut used, original.name.text.len())?;
        exports.push(zryna_abi::raw::Export::new(
            original.name.text.clone(),
            zryna_abi::raw::Signature::new(parameters, result),
        ));
        indices.push(index);
        selected[index] = true;
    }
    for (function, selected) in program.functions.iter().zip(selected) {
        charge(&mut used, 1)?;
        if function.executable_export.is_some() && !selected {
            return Err(reject("extra full-profile executable export claim"));
        }
    }
    // Independent inherited scalar-ABI issuance keeps its own published owner caps.
    // This ledger covers execution-owned source comparisons, raw rows/name/parameter
    // copies and retained selection indices above; it is not a transitive std-work
    // counter for private indexes/names inside the independently verified ABI object.
    // No execution seal is published by this phase. Failure drops all local outputs.
    let abi = zryna_abi::verify_v1(zryna_abi::raw::Module::new(exports))
        .map_err(|_| reject("full-profile exports violate the unchanged scalar ABI"))?;
    *spent = used;
    Ok((abi, indices))
}

fn find(program: &raw::Program, key: &[u8], spent: &mut usize) -> Result<usize, Failure> {
    let (mut low, mut high) = (0, program.functions.len());
    while low < high {
        let middle = low + (high - low) / 2;
        let candidate = &program.functions[middle].key;
        charge(spent, 1 + candidate.len().min(key.len()))?;
        match candidate.as_slice().cmp(key) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Ok(middle),
        }
    }
    Err(reject("original scalar export has no full-profile source root"))
}

fn source_scalar(
    unit: &RawSourceUnit,
    id: u32,
    original: &RawFunctionSyntax,
    sources: &SourceMap,
    spent: &mut usize,
) -> Result<zryna_abi::raw::Type, Failure> {
    charge(spent, 1)?;
    let node = unit.type_syntax.get(id as usize).ok_or(Failure::InternalFailure)?;
    if let RawTypeSyntaxKind::Named { name } = &node.kind {
        charge(spent, name.text.len())?;
        match name.text.as_str() {
            "bool" => return Ok(zryna_abi::raw::Type::Bool),
            "i32" => return Ok(zryna_abi::raw::Type::I32),
            _ => {}
        }
    }
    let span = sources.verify_span(original.name.span).map_err(|_| Failure::InternalFailure)?;
    Err(Failure::Diagnostics(vec![Diagnostic::error_at(
        "ZRYNA-M7005",
        span,
        "exported function signature cannot cross the selected scalar ABI",
        "keep generic and aggregate values internal; export a scalar wrapper",
    )]))
}

fn claimed_scalar(
    program: &raw::Program,
    ty: raw::ValueType,
    spent: &mut usize,
) -> Result<zryna_abi::raw::Type, Failure> {
    charge(spent, 1)?;
    let raw::ValueType::Stored(id) = ty else {
        return Err(reject("unit cannot enter the scalar export ABI"));
    };
    match program.type_keys.get(id as usize).map(Vec::as_slice) {
        Some([0]) => Ok(zryna_abi::raw::Type::Bool),
        Some([1]) => Ok(zryna_abi::raw::Type::I32),
        _ => Err(reject("non-scalar claim cannot enter the scalar export ABI")),
    }
}

#[cfg(test)]
mod tests;
