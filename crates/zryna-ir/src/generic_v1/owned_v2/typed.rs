use super::{Admission, Failure, graph, raw};
mod extension;
mod resources;
use crate::generic_v1::{body, cfg, inventory::Inventory, reject, reserve};
use zryna_layout::generic_v1::VerifiedLayouts;
use zryna_source::SourceMap;

pub(super) fn check(
    claim: &raw::Program,
    sources: &SourceMap,
    layouts: &VerifiedLayouts,
    inventory: &Inventory,
) -> Result<(), Failure> {
    check_for(claim, sources, layouts, inventory, Admission::LegacyOwned)
}

pub(super) fn check_for(
    claim: &raw::Program,
    sources: &SourceMap,
    layouts: &VerifiedLayouts,
    inventory: &Inventory,
    admission: Admission,
) -> Result<(), Failure> {
    if claim.extensions.len() != claim.graph.functions.len() {
        return Err(reject("incomplete extension inventory"));
    }
    let views = layouts.types().collect::<Vec<_>>();
    if views.iter().any(|ty| {
        !(matches!(ty.key().first(), Some(0 | 1 | 2 | 0x11 | 0x13 | 0x14 | 0x15))
            || admission == Admission::StructV4 && matches!(ty.key().first(), Some(0x10 | 0x12)))
    }) {
        return Err(reject("owned seal contains a type outside its admitted execution lane"));
    }
    resources::check(claim, admission)?;
    let mut calls = reserve(0)?;
    for (fi, function) in claim.graph.functions.iter().enumerate() {
        if matches!(function.result, graph::Type::Borrow { .. }) {
            return Err(reject("escaping function result loan"));
        }
        let g = cfg::build(function, sources, views.len())?;
        let ext = &claim.extensions[fi];
        if ext.windows(2).any(|p| p[0].result >= p[1].result) {
            return Err(reject("duplicate or unordered owned effects"));
        }
        let mut seen = 0;
        for (bi, block) in function.blocks.iter().enumerate() {
            for (pi, ins) in block.instructions.iter().enumerate() {
                let operand = |id| g.operand(id, bi, pi);
                let extended = ext
                    .binary_search_by_key(&ins.result.id, |e| e.result)
                    .ok()
                    .map(|i| &ext[i].operation);
                let expected = if let Some(op) = extended {
                    seen += 1;
                    if ins.operation != graph::Operation::Unit {
                        return Err(reject("owned effect lacks exact placeholder"));
                    }
                    extension::check(op, &views, &operand, admission)?
                } else {
                    body::operation(
                        &claim.graph,
                        &views,
                        inventory.generic_count,
                        &ins.operation,
                        &operand,
                    )?
                };
                if expected != ins.result.ty {
                    return Err(reject("owned operation result type mismatch"));
                }
                if let Some(target) =
                    body::call_target(&claim.graph, inventory.generic_count, &ins.operation)?
                {
                    calls.push((inventory.declarations[fi], inventory.declarations[target]));
                }
            }
            body::terminator(&views, function, &block.terminator, &|id| {
                g.operand(id, bi, block.instructions.len())
            })?;
        }
        if seen != ext.len() {
            return Err(reject("orphan owned effect claim"));
        }
    }
    crate::generic_v1::calls::check(claim.graph.declarations.len(), &calls)
}
