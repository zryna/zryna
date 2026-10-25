//! Exact complete operand roles; unfinished concrete rows reject in their owning phase.
use super::{Failure, Function, Kind, Site, SourceProof, TypeWorld, budget, frontier, raw, reject};
mod actions;
mod constructors;
mod containers;
mod push;
use actions::{clone_operation, loan, scalar};

#[derive(Clone, Copy)]
pub(super) struct Inputs<'a> {
    pub(super) program: &'a raw::Program,
    pub(super) source: &'a SourceProof,
    pub(super) frontier: &'a frontier::Table,
    pub(super) function: &'a Function,
    pub(super) site: Site,
    pub(super) callee: Option<usize>,
}
pub(super) fn check(
    input: Inputs<'_>,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<bool, Failure> {
    let Inputs { program: p, source, frontier, function: f, site, callee } = input;
    let world = source.world();
    let operand = |id| f.graph.operand(id, site.block, site.instruction);
    match &i.operation {
        raw::Operation::Unit => {
            if i.result.as_ref().is_some_and(|d| d.ty != raw::ValueType::Unit) {
                return Err(reject("Unit operation claims a stored result"));
            }
        }
        raw::Operation::BoolLiteral(_)
        | raw::Operation::I32Literal(_)
        | raw::Operation::I32Neg { .. }
        | raw::Operation::ScalarBinary { .. } => scalar(world, f, site, i, work)?,
        raw::Operation::ReadPlace { place, mode } => {
            let ty = f
                .places
                .rows
                .get(*place as usize)
                .ok_or_else(|| reject("read uses a foreign place"))?
                .ty;
            if *mode == raw::ReadMode::Copy
                && !world.fact(ty).ok_or_else(|| reject("place source fact absent"))?.copy
            {
                return Err(reject("Copy read requires source-derived Copy"));
            }
            result(i, Some(raw::ValueType::Stored(ty)))?;
        }
        raw::Operation::InitializePlace { place, value }
        | raw::Operation::ReplacePlace { place, value } => {
            let ty = f
                .places
                .rows
                .get(*place as usize)
                .ok_or_else(|| reject("write uses a foreign place"))?
                .ty;
            exact(operand(*value)?, ty)?;
            result(i, None)?;
        }
        raw::Operation::DropPlace { place } => {
            f.places
                .rows
                .get(*place as usize)
                .ok_or_else(|| reject("drop uses a foreign place"))?;
            result(i, None)?;
        }
        raw::Operation::Clone { .. } => {
            return clone_operation(source, frontier, p, f, site, i, work);
        }
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => {
            call(p, f, site, i, callee.ok_or_else(|| reject("call target absent"))?, arguments)?;
        }
        raw::Operation::StructConstruct { .. }
        | raw::Operation::ClosedEnumConstruct { .. }
        | raw::Operation::FixedArrayConstruct { .. } => {
            constructors::check(world, f, site, i, work)?;
        }
        raw::Operation::StringFromUtf8 { bytes, .. } => {
            if std::str::from_utf8(bytes).is_err() {
                return Err(reject("String literal is not canonical UTF-8"));
            }
            result(i, Some(raw::ValueType::Stored(primitive(world, Kind::String, work)?)))?;
            return Ok(true);
        }
        raw::Operation::IndexCopy { .. } => return actions::index(p, f, site, world, i, work),
        raw::Operation::BeginLoan { .. }
        | raw::Operation::EndLoan { .. }
        | raw::Operation::ReadLoan { .. }
        | raw::Operation::ReplaceLoan { .. } => return loan(p, f, site, world, i, work),
        raw::Operation::StringConcat { left, right, .. } => {
            let string = primitive(world, Kind::String, work)?;
            if referent(p, f, site, *left)? != string || referent(p, f, site, *right)? != string {
                return Err(reject("concat requires two exact String referents"));
            }
            result(i, Some(raw::ValueType::Stored(string)))?;
            return Ok(true);
        }
        raw::Operation::VecConstruct { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. }
        | raw::Operation::VecPush { .. } => {
            return containers::check(p, f, site, world, i, work);
        }
    }
    Ok(false)
}

pub(super) fn referent(
    p: &raw::Program,
    f: &Function,
    site: Site,
    source: raw::PlaceOrLoan,
) -> Result<u32, Failure> {
    match source {
        raw::PlaceOrLoan::Place(id) => f
            .places
            .rows
            .get(id as usize)
            .map(|p| p.ty)
            .ok_or_else(|| reject("source place is foreign")),
        raw::PlaceOrLoan::Loan(id) => p.functions[site.function]
            .loans
            .get(id as usize)
            .map(|l| l.referent)
            .ok_or_else(|| reject("source loan is foreign")),
    }
}
pub(super) fn result(
    i: &raw::Instruction,
    expected: Option<raw::ValueType>,
) -> Result<(), Failure> {
    if i.result.as_ref().map(|d| d.ty) != expected {
        return Err(reject("operation result binding/type differs from exact meaning"));
    }
    Ok(())
}
pub(super) fn exact(value: raw::ValueType, ty: u32) -> Result<(), Failure> {
    if value != raw::ValueType::Stored(ty) {
        return Err(reject("operand differs from exact stored type identity"));
    }
    Ok(())
}
pub(super) fn primitive(world: &TypeWorld, kind: Kind, work: &mut usize) -> Result<u32, Failure> {
    budget::charge(work, world.len())?;
    (0..u32::try_from(world.len()).map_err(|_| Failure::InternalFailure)?)
        .find(|id| world.kind(*id) == Some(kind))
        .ok_or_else(|| reject("required scalar type absent"))
}
fn call(
    p: &raw::Program,
    f: &Function,
    site: Site,
    i: &raw::Instruction,
    target: usize,
    arguments: &[raw::CallArgument],
) -> Result<(), Failure> {
    let callee = &p.functions[target];
    if arguments.len() != callee.parameters.len() {
        return Err(reject("call argument arity differs from exact private signature"));
    }
    for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
        match (argument, parameter) {
            (raw::CallArgument::Value(id), raw::Parameter::Value(d))
                if f.graph.operand(*id, site.block, site.instruction)? == d.ty => {}
            (raw::CallArgument::Loan(id), raw::Parameter::Loan(d)) => {
                let loan = p.functions[site.function]
                    .loans
                    .get(*id as usize)
                    .ok_or_else(|| reject("call loan is foreign"))?;
                if loan.referent != d.referent || loan.access != d.access {
                    return Err(reject("call loan differs from exact referent/access"));
                }
            }
            _ => return Err(reject("call value/loan role or type differs from exact signature")),
        }
    }
    result(i, Some(callee.result_type))
}
