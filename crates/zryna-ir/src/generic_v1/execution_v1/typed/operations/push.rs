//! Exact prepared append typing; source replay owns mutable binding permission.
use super::{
    Failure, Function, Kind, Site, TypeWorld, budget, exact, raw, referent, reject, result,
};
pub(super) fn check(
    p: &raw::Program,
    f: &Function,
    site: Site,
    world: &TypeWorld,
    i: &raw::Instruction,
    spent: &mut usize,
) -> Result<bool, Failure> {
    let mut work = *spent;
    let raw::Operation::VecPush { vector, value, .. } = i.operation else {
        return Err(reject("append typing requires exact VecPush opcode"));
    };
    budget::charge(&mut work, 2)?;
    if let raw::PlaceOrLoan::Loan(id) = vector {
        let loan = p.functions[site.function]
            .loans
            .get(id as usize)
            .ok_or_else(|| reject("append target loan is foreign"))?;
        if loan.access != raw::Access::Exclusive {
            return Err(reject("append requires exact exclusive loan access"));
        }
    }
    let ty = referent(p, f, site, vector)?;
    let element = super::containers::child(world, ty, Kind::Vec, &mut work)?;
    exact(f.graph.operand(value, site.block, site.instruction)?, element)?;
    result(i, None)?;
    *spent = work;
    Ok(true)
}
