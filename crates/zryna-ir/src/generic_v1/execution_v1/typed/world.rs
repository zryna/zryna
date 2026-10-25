//! Independent correspondence of source capability graph and both sealed physical universes.
use super::{Failure, SourceProof, budget, raw, reject};
use crate::generic_v1::execution_v1::type_world::{Kind, TypeWorld};
use zryna_layout::{
    TypeCategory,
    generic_v1::{TypeView, VerifiedLayouts},
};

pub(super) fn check(
    p: &raw::Program,
    source: &SourceProof,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    spent: &mut usize,
) -> Result<(), Failure> {
    let world = source.world();
    if world.len() != p.type_keys.len()
        || linear.types().len() != world.len()
        || linux.types().len() != world.len()
    {
        return Err(reject("execution source/type/layout universe inventories differ"));
    }
    budget::charge(
        spent,
        world
            .len()
            .checked_mul(4)
            .ok_or_else(|| super::resource_failure("execution type census overflow"))?,
    )?;
    for (index, (((key, raw), linear), linux)) in
        world.keys().zip(&p.type_keys).zip(linear.types()).zip(linux.types()).enumerate()
    {
        for candidate in [raw.as_slice(), linear.key(), linux.key()] {
            budget::charge(
                spent,
                key.len().min(candidate.len()).checked_add(1).ok_or_else(|| {
                    super::resource_failure("execution type key comparison overflow")
                })?,
            )?;
            if key != candidate {
                return Err(reject(
                    "execution type key differs from source or sealed target identity",
                ));
            }
        }
        let id = u32::try_from(index).map_err(|_| Failure::InternalFailure)?;
        record(world, id, linear, spent)?;
        record(world, id, linux, spent)?;
    }
    Ok(())
}

fn record(
    world: &TypeWorld,
    id: u32,
    view: TypeView<'_>,
    spent: &mut usize,
) -> Result<(), Failure> {
    let kind = world.kind(id).ok_or_else(|| reject("source type record absent"))?;
    let members = world.members(id).ok_or_else(|| reject("source member record absent"))?;
    budget::charge(
        spent,
        members
            .len()
            .checked_add(2)
            .ok_or_else(|| super::resource_failure("execution type member comparison overflow"))?,
    )?;
    if view.category() != category(kind)?
        || (world.fact(id).ok_or_else(|| reject("source fact absent"))?.copy
            && view.drop_kind() != 0)
    {
        return Err(reject("execution source type category/Copy differs from closed layout"));
    }
    match kind {
        Kind::Struct { .. } => {
            if view.fields().len() != members.len() {
                return Err(reject("Struct source/layout member count differs"));
            }
            for (index, ((ordinal, child, _), expected)) in view.fields().zip(members).enumerate() {
                if ordinal as usize != index || *expected != Some(child.index()) {
                    return Err(reject("Struct source/layout field identity/order differs"));
                }
            }
        }
        Kind::Enum { .. } | Kind::Option | Kind::Result => {
            if view.variants().len() != members.len() {
                return Err(reject("enum source/layout variant count differs"));
            }
            for (index, ((ordinal, child), expected)) in view.variants().zip(members).enumerate() {
                if ordinal as usize != index
                    || *expected != child.map(zryna_layout::generic_v1::TypeId::index)
                {
                    return Err(reject("enum source/layout payload identity/order differs"));
                }
            }
        }
        Kind::FixedArray { length } => {
            if view.array_length() != Some(u64::from(length))
                || members.len() != 1
                || members[0] != view.referenced_type().map(zryna_layout::generic_v1::TypeId::index)
                || view.array_stride().is_none()
            {
                return Err(reject("array source/layout length/element/stride differs"));
            }
        }
        Kind::Vec | Kind::Shared | Kind::Weak => {
            if members.len() != 1
                || members[0] != view.referenced_type().map(zryna_layout::generic_v1::TypeId::index)
            {
                return Err(reject("indirection source/layout referent differs"));
            }
        }
        Kind::Bool | Kind::I32 | Kind::String => {
            if !members.is_empty() {
                return Err(reject("source scalar has fabricated stored members"));
            }
        }
        Kind::Opaque { .. } => {
            return Err(reject(
                "opaque parameter cannot enter a closed physical execution universe",
            ));
        }
    }
    Ok(())
}
fn category(kind: Kind) -> Result<TypeCategory, Failure> {
    Ok(match kind {
        Kind::Bool => TypeCategory::Bool,
        Kind::I32 => TypeCategory::I32,
        Kind::String => TypeCategory::String,
        Kind::Struct { .. } => TypeCategory::Struct,
        Kind::Enum { .. } | Kind::Option | Kind::Result => TypeCategory::Enum,
        Kind::FixedArray { .. } => TypeCategory::FixedArray,
        Kind::Vec => TypeCategory::Vec,
        Kind::Shared => TypeCategory::Shared,
        Kind::Weak => TypeCategory::Weak,
        Kind::Opaque { .. } => {
            return Err(reject("opaque source atom is not a stored execution type"));
        }
    })
}

#[cfg(test)]
mod tests;
