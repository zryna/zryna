//! Deterministic checked frame geometry from exact new-domain views; no old IDs or seals.
use super::{Diagnostic, add, align, budget, invariant, raw};
use crate::generic_execution_v1::work::{retained, take as charge};
use zryna_ir::generic_v1::execution_v1::{
    OperationSite, VerifiedExecution, VerifiedFunction, VerifiedParameter,
    raw::{Operation, PlaceKind, ValueType},
};
use zryna_layout::{StorageTarget, generic_v1::TypeView};
type Inventory = (Vec<raw::FunctionFrame>, Vec<(OperationSite, Vec<u8>)>);
pub(super) fn inventory(
    program: &VerifiedExecution<'_>,
    work: &mut usize,
) -> Result<Inventory, Diagnostic> {
    charge(work, program.functions().len())?;
    let layout = program.layouts(StorageTarget::Linear32V1);
    charge(work, layout.types().len())?;
    retained::<TypeView<'_>>(work, layout.types().len())?;
    let mut types = Vec::new();
    #[cfg(test)]
    crate::generic_execution_v1::work::reserve_seen(layout.types().len());
    types.try_reserve_exact(layout.types().len()).map_err(|_| budget())?;
    types.extend(layout.types());
    retained::<raw::FunctionFrame>(work, program.functions().len())?;
    let mut functions = Vec::new();
    functions.try_reserve(program.functions().len()).map_err(|_| budget())?;
    let mut literals = Vec::new();
    let mut literal_bytes = 0usize;
    for f in program.functions() {
        functions.push(function(program, f, &types, work)?);
        charge(work, f.blocks().len())?;
        for block in f.blocks() {
            charge(work, block.operations().len())?;
            for op in block.operations() {
                // Until emitter and independent actual-byte audit consume exact partial targets,
                // a newly admitted partial old-drop receipt cannot reach whole-drop lowering.
                if op.drop_targets().is_some_and(|proof| !proof.is_whole()) {
                    return Err(invariant());
                }
                if let Operation::StringFromUtf8 { bytes, .. } = op.kind() {
                    literal_bytes = literal_bytes
                        .checked_add(bytes.len())
                        .filter(|n| *n <= super::MEMORY_BYTES as usize)
                        .ok_or_else(budget)?;
                    retained::<u8>(work, bytes.len())?;
                    charge(work, bytes.len())?;
                    retained::<(OperationSite, Vec<u8>)>(work, 1)?;
                    let mut value = Vec::new();
                    value.try_reserve_exact(bytes.len()).map_err(|_| budget())?;
                    value.extend_from_slice(bytes);
                    literals.try_reserve(1).map_err(|_| budget())?;
                    literals.push((op.site(), value));
                }
            }
        }
    }
    Ok((functions, literals))
}
fn function(
    _program: &VerifiedExecution<'_>,
    f: VerifiedFunction<'_, '_>,
    types: &[TypeView<'_>],
    work: &mut usize,
) -> Result<raw::FunctionFrame, Diagnostic> {
    let definitions = definitions(f, work)?;
    let count = definitions.len();
    let mut cursor = 0;
    retained::<raw::Slot>(work, count)?;
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| budget())?;
    for ty in definitions {
        let ty = ty.ok_or_else(invariant)?;
        let mut slot = slot(ty, types)?;
        cursor = align(cursor, slot.alignment)?;
        slot.offset = cursor;
        cursor = add(cursor, slot.bytes)?;
        values.push(slot);
    }
    retained::<raw::Slot>(work, f.places().len())?;
    let mut places: Vec<raw::Slot> = Vec::new();
    places.try_reserve_exact(f.places().len()).map_err(|_| budget())?;
    for p in f.places() {
        let mut s = slot(ValueType::Stored(p.ty()), types)?;
        s.offset = place_offset(
            f,
            p,
            &s,
            &Geometry { values: &values, places: &places, types },
            &mut cursor,
            work,
        )?;
        places.push(s);
    }
    let mut calls = Vec::new();
    charge(work, f.blocks().len())?;
    for b in f.blocks() {
        charge(work, b.operations().len())?;
        for o in b.operations() {
            if let Some(target) = o.call_target() {
                retained::<u32>(work, 1)?;
                calls.try_reserve(1).map_err(|_| budget())?;
                calls.push(u32::try_from(target.index()).map_err(|_| budget())?);
            }
        }
    }
    Ok(raw::FunctionFrame { bytes: align(cursor.max(1), 8)?, peak: 0, values, places, calls })
}
fn slot(ty: ValueType, types: &[TypeView<'_>]) -> Result<raw::Slot, Diagnostic> {
    let (bytes, alignment) = match ty {
        ValueType::Unit => (0, 1),
        ValueType::Stored(id) => {
            let t = types.get(id as usize).ok_or_else(invariant)?;
            (
                u32::try_from(t.size()).map_err(|_| budget())?,
                u32::try_from(t.alignment()).map_err(|_| budget())?,
            )
        }
    };
    if bytes > super::MEMORY_BYTES {
        return Err(budget());
    }
    Ok(raw::Slot { ty, offset: 0, bytes, alignment })
}
pub(super) fn claim_census(
    claim: &raw::TargetMemoryPlan<'_, '_>,
    work: &mut usize,
) -> Result<(), Diagnostic> {
    charge(work, claim.functions.len())?;
    charge(work, claim.literals.len())?;
    for f in &claim.functions {
        retained::<raw::Slot>(work, f.values.len())?;
        retained::<raw::Slot>(work, f.places.len())?;
        retained::<u32>(work, f.calls.len())?;
    }
    let mut bytes = 0usize;
    for literal in &claim.literals {
        charge(work, literal.bytes.len())?;
        bytes = bytes
            .checked_add(literal.bytes.len())
            .filter(|n| *n <= super::MEMORY_BYTES as usize)
            .ok_or_else(budget)?;
    }
    if claim.heap_start > super::MEMORY_BYTES {
        return Err(budget());
    }
    Ok(())
}

fn definitions(
    f: VerifiedFunction<'_, '_>,
    work: &mut usize,
) -> Result<Vec<Option<ValueType>>, Diagnostic> {
    charge(work, f.parameters().len())?;
    charge(work, f.blocks().len())?;
    let mut count = 0usize;
    for p in f.parameters() {
        if matches!(p, VerifiedParameter::Value(_)) {
            count = count.checked_add(1).ok_or_else(budget)?;
        } else {
            return Err(invariant());
        }
    }
    for b in f.blocks() {
        charge(work, b.operations().len())?;
        count = count.checked_add(b.value_parameters().len()).ok_or_else(budget)?;
        for o in b.operations() {
            count = count.checked_add(usize::from(o.result().is_some())).ok_or_else(budget)?;
        }
    }
    charge(
        work,
        count
            .checked_mul(3)
            .and_then(|n| n.checked_add(f.places().len().checked_mul(3)?))
            .ok_or_else(budget)?,
    )?;
    retained::<Option<ValueType>>(work, count)?;
    charge(work, f.parameters().len())?;
    charge(work, f.blocks().len())?;
    let mut definitions = Vec::new();
    definitions.try_reserve_exact(count).map_err(|_| budget())?;
    definitions.resize(count, None);
    let mut insert = |id: u32, ty: ValueType| -> Result<(), Diagnostic> {
        let slot = definitions.get_mut(id as usize).ok_or_else(invariant)?;
        if slot.replace(ty).is_some() {
            return Err(invariant());
        }
        Ok(())
    };
    for p in f.parameters() {
        if let VerifiedParameter::Value(v) = p {
            insert(v.id(), v.ty())?;
        }
    }
    for b in f.blocks() {
        charge(work, b.operations().len())?;
        for v in b.value_parameters() {
            insert(v.id(), v.ty())?;
        }
        for o in b.operations() {
            if let Some(v) = o.result() {
                insert(v.id(), v.ty())?;
            }
        }
    }
    Ok(definitions)
}

struct Geometry<'b, 'a> {
    values: &'b [raw::Slot],
    places: &'b [raw::Slot],
    types: &'b [TypeView<'a>],
}
fn place_offset(
    f: VerifiedFunction<'_, '_>,
    p: zryna_ir::generic_v1::execution_v1::VerifiedPlace<'_, '_>,
    s: &raw::Slot,
    geometry: &Geometry<'_, '_>,
    cursor: &mut u32,
    work: &mut usize,
) -> Result<u32, Diagnostic> {
    let values = geometry.values;
    let places = geometry.places;
    let types = geometry.types;
    let result = match p.kind() {
        PlaceKind::Parameter(ordinal) => {
            charge(work, ordinal as usize + 1)?;
            let VerifiedParameter::Value(v) =
                f.parameters().nth(ordinal as usize).ok_or_else(invariant)?
            else {
                return Err(invariant());
            };
            values[v.id() as usize].offset
        }
        PlaceKind::Temporary(id) => values.get(id as usize).ok_or_else(invariant)?.offset,
        PlaceKind::Local(_) => {
            *cursor = align(*cursor, s.alignment)?;
            let start = *cursor;
            *cursor = add(*cursor, s.bytes)?;
            start
        }
        PlaceKind::StructField { base, ordinal } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            charge(work, ordinal as usize + 1)?;
            add(
                parent.offset,
                u32::try_from(
                    types[ty as usize].fields().nth(ordinal as usize).ok_or_else(invariant)?.2,
                )
                .map_err(|_| budget())?,
            )?
        }
        PlaceKind::EnumPayload { base, .. } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            add(
                parent.offset,
                u32::try_from(types[ty as usize].payload().ok_or_else(invariant)?.0)
                    .map_err(|_| budget())?,
            )?
        }
        PlaceKind::FixedArrayConstant { base, index } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            let offset = types[ty as usize]
                .array_stride()
                .ok_or_else(invariant)?
                .checked_mul(u64::from(index))
                .ok_or_else(budget)?;
            add(parent.offset, u32::try_from(offset).map_err(|_| budget())?)?
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests;
