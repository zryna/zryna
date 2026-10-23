//! Exact new-domain place and loan indices; projections never cross an indirection.
use super::{
    Failure, budget, raw, reject, reserve, resource_failure,
    type_world::{Kind, TypeWorld},
};
pub(super) mod decomposition;
mod loans;

#[derive(Debug)]
pub(super) struct Row {
    pub(super) root: u32,
    pub(super) parent: Option<u32>,
    pub(super) ty: u32,
    pub(super) value: Option<u32>,
}
#[derive(Debug)]
pub(super) struct LoanRow {
    pub(super) root: Option<u32>,
    pub(super) parent: Option<u32>,
    pub(super) formal: bool,
    pub(super) anchor: u32,
    pub(super) local_place: Option<u32>,
}
#[derive(Debug)]
pub(super) struct Table {
    pub(super) decomposition: decomposition::Decomposition,
    pub(super) rows: Vec<Row>,
    pub(super) loans: Vec<LoanRow>,
    pub(super) owners: Vec<Option<u32>>,
}

pub(super) fn check(
    f: &raw::Function,
    types: &[raw::ValueType],
    world: &TypeWorld,
    spent: &mut usize,
) -> Result<Table, Failure> {
    let mut work = *spent;
    if f.places.len() > crate::data_ownership_v1::MAX_PLACES_PER_FUNCTION {
        return Err(resource_failure("execution place inventory exceeds inherited limit"));
    }
    budget::charge(
        &mut work,
        f.places
            .len()
            .checked_mul(5)
            .and_then(|n| n.checked_add(types.len()))
            .ok_or_else(|| resource_failure("execution place index overflow"))?,
    )?;
    let mut rows: Vec<Row> = reserve(f.places.len())?;
    let mut owners = reserve(types.len())?;
    owners.resize(types.len(), None);
    let parameters = parameter_values(f, types.len(), &mut work)?;
    let mut locals = reserve(f.places.len())?;
    locals.resize(f.places.len(), false);
    for (index, p) in f.places.iter().enumerate() {
        if p.id as usize != index || world.kind(p.ty).is_none() || !within(f.original_span, p.span)
        {
            return Err(reject("execution place identity/type/span is foreign"));
        }
        let (parent, owner) = match p.kind {
            raw::PlaceKind::Parameter(_)
            | raw::PlaceKind::Temporary(_)
            | raw::PlaceKind::Local(_) => {
                (None, root_owner(f, types, p, &mut locals, &parameters)?)
            }
            raw::PlaceKind::StructField { base, ordinal } => {
                let parent = prior(&rows, base)?;
                let child = projection(
                    world,
                    parent.ty,
                    raw::StaticProjection::StructField(ordinal),
                    &mut work,
                )?;
                if child != p.ty {
                    return Err(reject("Struct field place has a different exact type"));
                }
                (Some(base), None)
            }
            raw::PlaceKind::EnumPayload { base, variant } => {
                let parent = prior(&rows, base)?;
                let child = projection(
                    world,
                    parent.ty,
                    raw::StaticProjection::EnumPayload(variant),
                    &mut work,
                )?;
                if child != p.ty {
                    return Err(reject("enum payload place has a different exact type"));
                }
                (Some(base), None)
            }
            raw::PlaceKind::FixedArrayConstant { base, index } => {
                let parent = prior(&rows, base)?;
                let child = projection(
                    world,
                    parent.ty,
                    raw::StaticProjection::FixedArrayConstant(index),
                    &mut work,
                )?;
                if child != p.ty {
                    return Err(reject("array element place has a different exact type"));
                }
                (Some(base), None)
            }
        };
        if let Some(value) = owner {
            let slot = owners
                .get_mut(value as usize)
                .ok_or_else(|| reject("place binds a foreign value"))?;
            if slot.replace(p.id).is_some() {
                return Err(reject("prepared value has duplicate owning places"));
            }
        }
        rows.push(Row {
            root: parent.map_or(p.id, |id| rows[id as usize].root),
            parent,
            ty: p.ty,
            value: owner,
        });
    }
    verify_affine_owners(types, world, &owners)?;
    let mut loan_rows = loans::check(f, &rows, world, &mut work)?;
    let decomposition = decomposition::Decomposition::build(f, &rows, world, &mut work)?;
    loans::retain_referents(f, &decomposition, &mut loan_rows, &mut work)?;
    *spent = work;
    Ok(Table { rows, loans: loan_rows, owners, decomposition })
}

fn verify_affine_owners(
    types: &[raw::ValueType],
    world: &TypeWorld,
    owners: &[Option<u32>],
) -> Result<(), Failure> {
    for (id, ty) in types.iter().enumerate() {
        if let raw::ValueType::Stored(ty) = ty
            && !world.fact(*ty).ok_or_else(|| reject("value lacks source capability fact"))?.copy
            && owners[id].is_none()
        {
            return Err(reject("affine prepared value lacks its unique pending temporary place"));
        }
    }
    Ok(())
}

pub(super) fn projection(
    world: &TypeWorld,
    ty: u32,
    p: raw::StaticProjection,
    work: &mut usize,
) -> Result<u32, Failure> {
    budget::charge(work, 2)?;
    let kind = world.kind(ty).ok_or_else(|| reject("projection base has no source type"))?;
    let members =
        world.members(ty).ok_or_else(|| reject("projection base has no source members"))?;
    let ordinal = match (p, kind) {
        (raw::StaticProjection::StructField(i), Kind::Struct { .. })
        | (
            raw::StaticProjection::EnumPayload(i),
            Kind::Enum { .. } | Kind::Option | Kind::Result,
        ) => i as usize,
        (raw::StaticProjection::FixedArrayConstant(i), Kind::FixedArray { length })
            if i < length =>
        {
            0
        }
        _ => return Err(reject("projection kind/bounds do not match exact source type")),
    };
    members
        .get(ordinal)
        .copied()
        .flatten()
        .ok_or_else(|| reject("projection selects an absent member/payload"))
}
fn prior(rows: &[Row], id: u32) -> Result<&Row, Failure> {
    rows.get(id as usize).ok_or_else(|| reject("projection parent is forward/cyclic/foreign"))
}
pub(super) fn within(
    outer: zryna_source::UntrustedSpan,
    inner: zryna_source::UntrustedSpan,
) -> bool {
    outer.file == inner.file
        && outer.start <= inner.start
        && inner.start <= inner.end
        && inner.end <= outer.end
}

fn root_owner(
    f: &raw::Function,
    types: &[raw::ValueType],
    p: &raw::Place,
    locals: &mut [bool],
    parameters: &[bool],
) -> Result<Option<u32>, Failure> {
    Ok(match p.kind {
        raw::PlaceKind::Parameter(ordinal) => {
            let Some(raw::Parameter::Value(d)) = f.parameters.get(ordinal as usize) else {
                return Err(reject("place parameter is not an ordinary signature value"));
            };
            if d.ty != raw::ValueType::Stored(p.ty) {
                return Err(reject("parameter place type differs from signature"));
            }
            Some(d.id)
        }
        raw::PlaceKind::Temporary(value) => {
            if parameters.get(value as usize) == Some(&true) {
                return Err(reject("signature value cannot masquerade as pending temporary"));
            }
            if types.get(value as usize) != Some(&raw::ValueType::Stored(p.ty)) {
                return Err(reject("temporary place does not bind its exact prepared value"));
            }
            Some(value)
        }
        raw::PlaceKind::Local(ordinal) => {
            let slot = locals
                .get_mut(ordinal as usize)
                .ok_or_else(|| reject("local ordinal exceeds place census"))?;
            if std::mem::replace(slot, true) {
                return Err(reject("duplicate local place ordinal"));
            }
            None
        }
        _ => return Err(reject("projection has no direct SSA owner")),
    })
}

fn parameter_values(
    f: &raw::Function,
    count: usize,
    work: &mut usize,
) -> Result<Vec<bool>, Failure> {
    budget::charge(
        work,
        count
            .checked_add(f.parameters.len())
            .ok_or_else(|| resource_failure("execution parameter owner census overflow"))?,
    )?;
    let mut parameters = reserve(count)?;
    parameters.resize(count, false);
    for parameter in &f.parameters {
        if let raw::Parameter::Value(d) = parameter {
            parameters[d.id as usize] = true;
        }
    }
    Ok(parameters)
}
