use super::{Failure, Function, Kind, Site, TypeWorld, budget, exact, raw, reject, result};

pub(super) fn check(
    world: &TypeWorld,
    f: &Function,
    site: Site,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<(), Failure> {
    let operand = |id| f.graph.operand(id, site.block, site.instruction);
    match &i.operation {
        raw::Operation::StructConstruct { ty, fields } => {
            if !matches!(world.kind(*ty), Some(Kind::Struct { .. })) {
                return Err(reject(
                    "Struct construction cannot substitute an array or equal-size nominal",
                ));
            }
            let members = world.members(*ty).ok_or_else(|| reject("Struct member proof absent"))?;
            if fields.len() != members.len() {
                return Err(reject("Struct constructor member count differs"));
            }
            budget::charge(work, fields.len())?;
            for (id, ty) in fields.iter().zip(members) {
                exact(
                    operand(*id)?,
                    ty.ok_or_else(|| reject("Struct member has no stored type"))?,
                )?;
            }
            result(i, Some(raw::ValueType::Stored(*ty)))
        }
        raw::Operation::ClosedEnumConstruct { ty, ordinal, payload } => {
            if !matches!(world.kind(*ty), Some(Kind::Enum { .. } | Kind::Option | Kind::Result)) {
                return Err(reject("enum construction requires exact nominal/family type"));
            }
            let expected = world
                .members(*ty)
                .and_then(|m| m.get(*ordinal as usize))
                .ok_or_else(|| reject("enum ordinal absent"))?;
            match (payload, expected) {
                (None, None) => {}
                (Some(id), Some(ty)) => exact(operand(*id)?, *ty)?,
                _ => return Err(reject("enum payload presence differs from exact active variant")),
            }
            result(i, Some(raw::ValueType::Stored(*ty)))
        }
        raw::Operation::FixedArrayConstruct { ty, elements } => {
            let Some(Kind::FixedArray { length }) = world.kind(*ty) else {
                return Err(reject("array construction cannot substitute a Struct carrier"));
            };
            let element = world
                .members(*ty)
                .and_then(|m| m.first())
                .copied()
                .flatten()
                .ok_or_else(|| reject("array element proof absent"))?;
            if elements.len() != length as usize {
                return Err(reject("array constructor count differs from exact declared length"));
            }
            budget::charge(work, elements.len())?;
            for id in elements {
                exact(operand(*id)?, element)?;
            }
            result(i, Some(raw::ValueType::Stored(*ty)))
        }
        _ => Err(reject("constructor checker received another operation")),
    }
}
