//! Concrete container preparation typing; exact source-world identities never become layout flags.
use super::{
    Failure, Function, Kind, Site, TypeWorld, budget, exact, raw, referent, reject, result,
};
#[cfg(test)]
mod tests;

pub(super) fn check(
    p: &raw::Program,
    f: &Function,
    site: Site,
    world: &TypeWorld,
    i: &raw::Instruction,
    spent: &mut usize,
) -> Result<bool, Failure> {
    let mut work = *spent;
    let operand = |id| f.graph.operand(id, site.block, site.instruction);
    match &i.operation {
        raw::Operation::VecConstruct { ty, elements, .. } => {
            let element = child(world, *ty, Kind::Vec, &mut work)?;
            result(i, Some(raw::ValueType::Stored(*ty)))?;
            budget::charge(&mut work, elements.len())?;
            for value in elements {
                exact(operand(*value)?, element)?;
            }
        }
        raw::Operation::SharedConstruct { ty, value, .. } => {
            let payload = child(world, *ty, Kind::Shared, &mut work)?;
            exact(operand(*value)?, payload)?;
            result(i, Some(raw::ValueType::Stored(*ty)))?;
        }
        raw::Operation::WeakDowngrade { shared, .. } => {
            let source = referent(p, f, site, *shared)?;
            let payload = child(world, source, Kind::Shared, &mut work)?;
            let Some(raw::ValueDefinition { ty: raw::ValueType::Stored(ty), .. }) = &i.result
            else {
                return Err(reject("downgrade requires an exact stored Weak result"));
            };
            if child(world, *ty, Kind::Weak, &mut work)? != payload {
                return Err(reject("downgrade changes exact Shared/Weak referent identity"));
            }
        }
        raw::Operation::VecPush { .. } => return super::push::check(p, f, site, world, i, spent),
        _ => return Err(reject("container checker requires an exact settled preparation opcode")),
    }
    *spent = work;
    Ok(true)
}
pub(super) fn child(
    world: &TypeWorld,
    ty: u32,
    expected: Kind,
    work: &mut usize,
) -> Result<u32, Failure> {
    budget::charge(work, 3)?;
    if world.kind(ty) != Some(expected) {
        return Err(reject("container opcode has a different exact source type kind"));
    }
    let [child] = world.arguments(ty).ok_or_else(|| reject("container argument proof absent"))?
    else {
        return Err(reject("container requires exactly one original argument"));
    };
    if world.members(ty) != Some([Some(*child)].as_slice()) {
        return Err(reject("container source member differs from its exact argument"));
    }
    Ok(*child)
}

#[cfg(test)]
mod push_tests;
