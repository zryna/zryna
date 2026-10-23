use super::{
    Failure, Function, Kind, Site, SourceProof, TypeWorld, exact, frontier, primitive, raw,
    referent, reject, result,
};

pub(super) fn scalar(
    world: &TypeWorld,
    f: &Function,
    site: Site,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<(), Failure> {
    let operand = |id| f.graph.operand(id, site.block, site.instruction);
    match &i.operation {
        raw::Operation::BoolLiteral(_) => {
            result(i, Some(raw::ValueType::Stored(primitive(world, Kind::Bool, work)?)))?;
        }
        raw::Operation::I32Literal(_) => {
            result(i, Some(raw::ValueType::Stored(primitive(world, Kind::I32, work)?)))?;
        }
        raw::Operation::I32Neg { value } => {
            let ty = primitive(world, Kind::I32, work)?;
            exact(operand(*value)?, ty)?;
            result(i, Some(raw::ValueType::Stored(ty)))?;
        }
        raw::Operation::ScalarBinary { kind, left, right } => {
            let left = operand(*left)?;
            let right = operand(*right)?;
            if left != right {
                return Err(reject("scalar operands have different exact types"));
            }
            let raw::ValueType::Stored(ty) = left else {
                return Err(reject("unit is not a scalar operand"));
            };
            let comparison = matches!(
                kind,
                raw::ScalarBinaryKind::Eq
                    | raw::ScalarBinaryKind::Ne
                    | raw::ScalarBinaryKind::LtS
                    | raw::ScalarBinaryKind::LeS
                    | raw::ScalarBinaryKind::GtS
                    | raw::ScalarBinaryKind::GeS
            );
            let equality = matches!(kind, raw::ScalarBinaryKind::Eq | raw::ScalarBinaryKind::Ne);
            if world.kind(ty) != Some(Kind::I32)
                && !(equality && world.kind(ty) == Some(Kind::Bool))
            {
                return Err(reject("scalar opcode does not match exact scalar type"));
            }
            let out = if comparison { primitive(world, Kind::Bool, work)? } else { ty };
            result(i, Some(raw::ValueType::Stored(out)))?;
        }
        _ => return Err(reject("scalar checker received a nonscalar operation")),
    }
    Ok(())
}
pub(super) fn clone_operation(
    source: &SourceProof,
    frontier: &frontier::Table,
    p: &raw::Program,
    f: &Function,
    site: Site,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<bool, Failure> {
    let world = source.world();
    match &i.operation {
        raw::Operation::Clone { source: src, .. } => {
            let ty = referent(p, f, site, *src)?;
            if !world.fact(ty).ok_or_else(|| reject("clone source fact absent"))?.clone
                || !source.clone_allowed(
                    site.function,
                    u32::try_from(site.block).map_err(|_| Failure::InternalFailure)?,
                    u32::try_from(site.instruction).map_err(|_| Failure::InternalFailure)?,
                    ty,
                    work,
                )?
            {
                return Err(reject(
                    "Clone lacks independent original source capability/site proof",
                ));
            }
            clone_shape(frontier, ty, i)
        }
        _ => Err(reject("clone checker received a nonclone operation")),
    }
}

// This post-source-permission contract is shared by production and narrow owning controls.
fn clone_shape(frontier: &frontier::Table, ty: u32, i: &raw::Instruction) -> Result<bool, Failure> {
    let raw::Operation::Clone { outer_failure, prefix_failure, .. } = &i.operation else {
        return Err(reject("clone shape checker received a nonclone operation"));
    };
    result(i, Some(raw::ValueType::Stored(ty)))?;
    let falls = frontier.node(ty)?.may_fail;
    if outer_failure.is_some() != falls || prefix_failure.is_some() != falls {
        return Err(reject("clone failure claims differ from exact derived frontier"));
    }
    Ok(falls)
}
#[cfg(test)]
mod tests;

pub(super) fn loan(
    p: &raw::Program,
    f: &Function,
    site: Site,
    world: &TypeWorld,
    i: &raw::Instruction,
) -> Result<(), Failure> {
    match &i.operation {
        raw::Operation::BeginLoan { loan, failure } => {
            let row = p.functions[site.function]
                .loans
                .get(*loan as usize)
                .ok_or_else(|| reject("BeginLoan has a foreign loan"))?;
            if failure.is_some() || matches!(row.origin, raw::LoanOrigin::Indexed { .. }) {
                return Err(reject("indexed loan preparation is not implemented yet"));
            }
            result(i, None)?;
        }
        raw::Operation::EndLoan { loan } => {
            f.places
                .loans
                .get(*loan as usize)
                .ok_or_else(|| reject("EndLoan has a foreign loan"))?;
            result(i, None)?;
        }
        raw::Operation::ReadLoan { loan } => {
            let ty = referent(p, f, site, raw::PlaceOrLoan::Loan(*loan))?;
            if !world.fact(ty).ok_or_else(|| reject("loan source fact absent"))?.copy {
                return Err(reject("loan read requires original Copy"));
            }
            result(i, Some(raw::ValueType::Stored(ty)))?;
        }
        raw::Operation::ReplaceLoan { loan, value } => {
            let ty = referent(p, f, site, raw::PlaceOrLoan::Loan(*loan))?;
            let row = &p.functions[site.function].loans[*loan as usize];
            if row.access != raw::Access::Exclusive {
                return Err(reject("loan replacement requires exact exclusive access"));
            }
            exact(f.graph.operand(*value, site.block, site.instruction)?, ty)?;
            result(i, None)?;
        }
        _ => return Err(reject("loan checker requires exact loan opcode")),
    }
    Ok(())
}

pub(super) fn index(
    p: &raw::Program,
    f: &Function,
    site: Site,
    world: &TypeWorld,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<bool, Failure> {
    let operand = |id| f.graph.operand(id, site.block, site.instruction);
    match &i.operation {
        raw::Operation::IndexCopy { base, index, .. } => {
            exact(operand(*index)?, primitive(world, Kind::I32, work)?)?;
            let ty = referent(p, f, site, *base)?;
            if !matches!(world.kind(ty), Some(Kind::FixedArray { .. } | Kind::Vec)) {
                return Err(reject("index base is not an exact array/Vec"));
            }
            let element = world
                .members(ty)
                .and_then(|m| m.first())
                .copied()
                .flatten()
                .ok_or_else(|| reject("index element type absent"))?;
            if !world.fact(element).ok_or_else(|| reject("index source fact absent"))?.copy {
                return Err(reject("dynamic index cannot implicitly duplicate an affine element"));
            }
            result(i, Some(raw::ValueType::Stored(element)))?;
            Ok(true)
        }
        _ => Err(reject("index checker requires exact IndexCopy opcode")),
    }
}
