//! Release calls are counted independently from lane lowering and emission work caches.
use super::{Layout, parse_error, reject};
use crate::generic_owned_v2::{budget, effects::value_type};
use wasmparser::{Operator, Parser, Payload};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::{owned_v2::raw::Operation, raw};
use zryna_layout::{StorageTarget, TypeCategory, generic_v1::TypeView};

pub(super) fn check(bytes: &[u8], layout: &Layout<'_, '_>) -> Result<(), Diagnostic> {
    let views = layout.program.layouts(StorageTarget::Linear32V1);
    if !views.types().any(|ty| ty.category() == TypeCategory::Struct) {
        return Ok(());
    }
    let mut work = 0usize;
    add(&mut work, views.types().len().checked_mul(3).ok_or_else(budget)?)?;
    let mut types = Vec::new();
    types.try_reserve(views.types().len()).map_err(|_| budget())?;
    types.extend(views.types());
    let mut cache = Vec::new();
    cache.try_reserve(types.len()).map_err(|_| budget())?;
    cache.resize(types.len(), None);
    let mut index = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::CodeSectionEntry(body) = payload.map_err(parse_error)? {
            let mut expected = 0usize;
            if let Some(function) = layout.program.functions().get(index) {
                for block in &function.blocks {
                    for i in &block.instructions {
                        if let Some(Operation::Drop(id)) =
                            layout.program.extension(index, i.result.id)
                        {
                            add(
                                &mut expected,
                                releases(
                                    value_type(function, *id)?,
                                    &types,
                                    &mut cache,
                                    0,
                                    &mut work,
                                )?,
                            )?;
                        }
                    }
                }
                if let Some(plan) = layout.program.plan(index) {
                    for step in &plan.steps {
                        for id in &step.cleanup {
                            add(
                                &mut expected,
                                releases(
                                    value_type(function, *id)?,
                                    &types,
                                    &mut cache,
                                    0,
                                    &mut work,
                                )?,
                            )?;
                        }
                    }
                }
            }
            let mut actual = 0;
            let mut operators = body.get_operators_reader().map_err(parse_error)?;
            while !operators.eof() {
                if matches!(
                    operators.read().map_err(parse_error)?,
                    Operator::Call { function_index: 2 }
                ) {
                    add(&mut actual, 1)?;
                }
            }
            if actual != expected {
                return Err(reject("changed recursive release call census"));
            }
            index += 1;
        }
    }
    Ok(())
}
fn add(total: &mut usize, amount: usize) -> Result<(), Diagnostic> {
    *total = total
        .checked_add(amount)
        .filter(|n| *n <= crate::generic_owned_v2::MAX_BYTES)
        .ok_or_else(budget)?;
    Ok(())
}
fn releases(
    ty: raw::Type,
    types: &[TypeView<'_>],
    cache: &mut [Option<usize>],
    depth: usize,
    work: &mut usize,
) -> Result<usize, Diagnostic> {
    add(work, 1)?;
    let raw::Type::Stored(index) = ty else {
        return Err(reject("nonstored release authority"));
    };
    if let Some(count) = cache.get(index as usize).copied().flatten() {
        return Ok(count);
    }
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget());
    }
    let ty = types.get(index as usize).copied().ok_or_else(|| reject("missing release type"))?;
    let mut count = 0;
    if ty.drop_kind() != 0 {
        match ty.category() {
            TypeCategory::String => count = 1,
            TypeCategory::Struct => {
                add(work, ty.fields().len())?;
                for (_, field, _) in ty.fields() {
                    add(
                        &mut count,
                        releases(raw::Type::Stored(field.index()), types, cache, depth + 1, work)?,
                    )?;
                }
            }
            TypeCategory::Enum => {
                add(work, ty.variants().len())?;
                for (_, payload) in ty.variants() {
                    if let Some(payload) = payload {
                        add(
                            &mut count,
                            releases(
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
            _ => return Err(reject("unsupported recursive release type")),
        }
    }
    cache[index as usize] = Some(count);
    Ok(count)
}
