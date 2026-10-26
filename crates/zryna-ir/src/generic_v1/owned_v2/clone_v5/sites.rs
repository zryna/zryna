//! Each retained site binds an exact Unit result, source identity and owner-plan location.
use super::recipe::Builder;
use super::{Failure, Site, Table, VerifiedLayouts, graph, plan, raw, reject, reserve};
#[derive(Clone, Copy)]
struct Definition {
    ty: graph::Type,
    location: Option<(u32, u32)>,
    unit: bool,
}
impl Table {
    pub(in crate::generic_v1::owned_v2) fn derive(
        program: &graph::Program,
        extensions: &[Vec<raw::Extension>],
        layouts: &VerifiedLayouts,
        used: &mut usize,
    ) -> Result<Self, Failure> {
        if extensions.len() != program.functions.len() {
            return Err(reject("clone table effect inventory differs"));
        }
        let mut builder = Builder::new(layouts, used)?;
        let mut sites = Vec::new();
        for (function, (body, effects)) in program.functions.iter().zip(extensions).enumerate() {
            plan::charge(used, 1)?;
            let definitions = definitions(body, used)?;
            for effect in effects {
                plan::charge(used, 1)?;
                let raw::Operation::CloneStruct { value } = &effect.operation else { continue };
                let source = definitions
                    .get(*value as usize)
                    .ok_or_else(|| reject("clone source is absent"))?;
                let result = definitions
                    .get(effect.result as usize)
                    .ok_or_else(|| reject("clone result is absent"))?;
                let graph::Type::Stored(ty) = source.ty else {
                    return Err(reject("Struct clone requires a stored source"));
                };
                if result.ty != source.ty || !result.unit {
                    return Err(reject("clone result is not its exact same-type Unit placeholder"));
                }
                let (block, position) =
                    result.location.ok_or_else(|| reject("clone result cannot be a parameter"))?;
                let root = builder.root(ty, used)?;
                plan::charge(
                    used,
                    usize::try_from(builder.nodes[root as usize].work)
                        .map_err(|_| Failure::InternalFailure)?,
                )?;
                plan::charge(used, 1)?;
                sites.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                sites.push(Site {
                    function,
                    result: effect.result,
                    source: *value,
                    ty,
                    root,
                    block,
                    position,
                    failure: None,
                });
            }
        }
        Ok(Self { nodes: builder.nodes, sites })
    }
    pub(in crate::generic_v1::owned_v2) fn validate_plans(
        &mut self,
        plans: &[raw::Plan],
        used: &mut usize,
    ) -> Result<(), Failure> {
        for site in &mut self.sites {
            plan::charge(used, 1)?;
            let steps = &plans.get(site.function).ok_or(Failure::InternalFailure)?.steps;
            let mut first = 0;
            let mut end = steps.len();
            let mut found = None;
            while first < end {
                let middle = first + (end - first) / 2;
                plan::charge(used, 1)?;
                let step = &steps[middle];
                match (step.block, step.position, step.failure).cmp(&(
                    site.block,
                    site.position,
                    true,
                )) {
                    std::cmp::Ordering::Less => first = middle + 1,
                    std::cmp::Ordering::Greater => end = middle,
                    std::cmp::Ordering::Equal => {
                        found = Some(middle);
                        break;
                    }
                }
            }
            let fallible = self.nodes[site.root as usize].strings != 0;
            if let Some(index) = found {
                let cleanup = &steps[index].cleanup;
                plan::charge(used, cleanup.len().checked_mul(2).ok_or(Failure::InternalFailure)?)?;
                if cleanup.contains(&site.result) || !cleanup.contains(&site.source) {
                    return Err(reject(
                        "Struct clone snapshot does not retain source and exclude result",
                    ));
                }
            }
            if fallible != found.is_some() {
                return Err(reject(
                    "Struct clone failure step does not exclude the unwritten result",
                ));
            }
            site.failure = found;
        }
        Ok(())
    }
}
fn definitions(function: &graph::Function, used: &mut usize) -> Result<Vec<Definition>, Failure> {
    plan::charge(used, function.blocks.len())?;
    let count = function
        .blocks
        .iter()
        .try_fold(0usize, |n, b| {
            n.checked_add(b.parameters.len())?.checked_add(b.instructions.len())
        })
        .ok_or(Failure::InternalFailure)?;
    plan::charge(used, count.checked_mul(2).ok_or(Failure::InternalFailure)?)?;
    let mut rows = reserve(count)?;
    for block in &function.blocks {
        for parameter in &block.parameters {
            push(&mut rows, parameter, None, false)?;
        }
        for (position, instruction) in block.instructions.iter().enumerate() {
            push(
                &mut rows,
                &instruction.result,
                Some((block.id, u32::try_from(position).map_err(|_| Failure::InternalFailure)?)),
                instruction.operation == graph::Operation::Unit,
            )?;
        }
    }
    Ok(rows)
}
fn push(
    rows: &mut Vec<Definition>,
    definition: &graph::Definition,
    location: Option<(u32, u32)>,
    unit: bool,
) -> Result<(), Failure> {
    if usize::try_from(definition.id).ok() != Some(rows.len()) {
        return Err(reject("clone definitions are not dense"));
    }
    rows.push(Definition { ty: definition.ty, location, unit });
    Ok(())
}
