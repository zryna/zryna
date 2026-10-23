//! Preflight new field traversal and retention before any backend expansion.
use super::{MAX_BYTES, budget, effects::value_type, invariant};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::{
    owned_v2::{VerifiedOwnedProgram, raw::Operation},
    raw,
};
use zryna_layout::{StorageTarget, TypeCategory, generic_v1::TypeView};

pub(super) fn check(program: &VerifiedOwnedProgram<'_>) -> Result<(), Diagnostic> {
    let layouts = program.layouts(StorageTarget::Linear32V1);
    if !layouts.types().any(|ty| ty.category() == TypeCategory::Struct) {
        return Ok(());
    }
    let mut work = 0usize;
    charge(&mut work, layouts.types().len().checked_mul(3).ok_or_else(budget)?)?;
    for ty in layouts.types() {
        if ty.category() == TypeCategory::Struct {
            charge(&mut work, ty.fields().len())?;
        }
    }
    let mut types = Vec::new();
    types.try_reserve(layouts.types().len()).map_err(|_| budget())?;
    types.extend(layouts.types());
    let mut costs = Vec::new();
    costs.try_reserve(types.len()).map_err(|_| budget())?;
    costs.resize(types.len(), None);
    for (fi, f) in program.functions().iter().enumerate() {
        charge(&mut work, f.blocks.len())?;
        let definitions = f.blocks.iter().try_fold(0usize, |n, block| {
            n.checked_add(block.parameters.len())
                .and_then(|n| n.checked_add(block.instructions.len()))
                .ok_or_else(budget)
        })?;
        for block in &f.blocks {
            for i in &block.instructions {
                match program.extension(fi, i.result.id) {
                    Some(Operation::StructConstruct { ty, fields }) => {
                        charge(&mut work, (*ty as usize).checked_add(1).ok_or_else(budget)?)?;
                        charge(&mut work, fields.len().checked_mul(3).ok_or_else(budget)?)?;
                    }
                    Some(Operation::CopyStructField { value, ordinal }) => {
                        let raw::Type::Stored(index) =
                            lookup(f, *value, definitions, 2, &mut work)?
                        else {
                            return Err(invariant());
                        };
                        let view = types.get(index as usize).ok_or_else(invariant)?;
                        charge(&mut work, view.fields().len().checked_mul(3).ok_or_else(budget)?)?;
                        charge(&mut work, (index as usize).checked_add(1).ok_or_else(budget)?)?;
                        let (_, field, _) =
                            view.fields().nth(*ordinal as usize).ok_or_else(invariant)?;
                        charge(
                            &mut work,
                            (field.index() as usize).checked_add(1).ok_or_else(budget)?,
                        )?;
                    }
                    Some(Operation::Drop(id)) => {
                        let ty = lookup(f, *id, definitions, 3, &mut work)?;
                        let cost = drop_cost(ty, &types, &mut costs, 0, &mut work)?;
                        charge(&mut work, cost)?;
                    }
                    _ => {}
                }
            }
        }
        if let Some(plan) = program.plan(fi) {
            for step in &plan.steps {
                for id in &step.cleanup {
                    let ty = lookup(f, *id, definitions, 3, &mut work)?;
                    let cost = drop_cost(ty, &types, &mut costs, 0, &mut work)?;
                    charge(&mut work, cost)?;
                }
            }
        }
    }
    Ok(())
}
fn charge(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    *work = work.checked_add(count).filter(|n| *n <= MAX_BYTES).ok_or_else(budget)?;
    Ok(())
}
fn drop_cost(
    ty: raw::Type,
    types: &[TypeView<'_>],
    cache: &mut [Option<usize>],
    depth: usize,
    work: &mut usize,
) -> Result<usize, Diagnostic> {
    charge(work, 1)?;
    let raw::Type::Stored(index) = ty else {
        return Err(invariant());
    };
    if let Some(cost) = cache.get(index as usize).copied().flatten() {
        return Ok(cost);
    }
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget());
    }
    let view = types.get(index as usize).copied().ok_or_else(invariant)?;
    let mut cost = (index as usize).checked_add(1).ok_or_else(budget)?;
    if view.drop_kind() != 0 {
        match view.category() {
            TypeCategory::Struct => {
                charge(work, view.fields().len())?;
                // drop_value and fields each resolve this type through nth.
                charge(&mut cost, (index as usize).checked_add(1).ok_or_else(budget)?)?;
                charge(&mut cost, view.fields().len().checked_mul(3).ok_or_else(budget)?)?;
                for (_, field, _) in view.fields() {
                    charge(
                        &mut cost,
                        drop_cost(raw::Type::Stored(field.index()), types, cache, depth + 1, work)?,
                    )?;
                }
            }
            TypeCategory::Enum => {
                charge(work, view.variants().len())?;
                charge(&mut cost, view.variants().len())?;
                for (_, payload) in view.variants() {
                    if let Some(payload) = payload {
                        charge(
                            &mut cost,
                            drop_cost(
                                raw::Type::Stored(payload.index()),
                                types,
                                cache,
                                depth + 1,
                                work,
                            )?,
                        )?;
                    }
                }
            }
            TypeCategory::String => {}
            _ => return Err(invariant()),
        }
    }
    cache[index as usize] = Some(cost);
    Ok(cost)
}

fn lookup(
    f: &raw::Function,
    id: u32,
    definitions: usize,
    passes: usize,
    work: &mut usize,
) -> Result<raw::Type, Diagnostic> {
    // Full definition census precedes this scan and its later emitter/auditor scans.
    charge(work, definitions.checked_mul(passes).ok_or_else(budget)?)?;
    value_type(f, id)
}
