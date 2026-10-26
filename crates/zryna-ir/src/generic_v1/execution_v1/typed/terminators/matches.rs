//! Complete closed identity and exact mode-specific selected binding schema.
use super::{Failure, Function, Kind, TypeWorld, budget, raw, reject};
pub(super) fn check(
    f: &raw::Function,
    checked: &Function,
    world: &TypeWorld,
    t: &raw::Terminator,
    work: &mut usize,
) -> Result<(), Failure> {
    let raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } = t else {
        return Err(Failure::InternalFailure);
    };
    if !matches!(world.kind(*ty), Some(Kind::Enum { .. } | Kind::Option | Kind::Result)) {
        return Err(reject("match requires an exact enum/family identity"));
    }
    let actual = match scrutinee {
        raw::PlaceOrLoan::Place(id) => checked.places.rows.get(*id as usize).map(|p| p.ty),
        raw::PlaceOrLoan::Loan(id) => f.loans.get(*id as usize).map(|l| l.referent),
    };
    if actual != Some(*ty) {
        return Err(reject("match source type differs from full enum identity"));
    }
    let members = world.members(*ty).ok_or_else(|| reject("match variant proof absent"))?;
    budget::charge(work, members.len())?;
    if arms.len() != members.len() {
        return Err(reject("match does not cover every exact variant"));
    }
    if *mode != raw::MatchMode::Value {
        budget::charge(work, arms.len().checked_mul(4).ok_or(Failure::InternalFailure)?)?;
    }
    for (ordinal, (arm, payload)) in arms.iter().zip(members).enumerate() {
        if arm.ordinal as usize != ordinal {
            return Err(reject("match ordinal order is duplicated or foreign"));
        }
        match mode {
            raw::MatchMode::Value => match (&arm.binding, payload) {
                (None, None) => {}
                (Some(raw::MatchBinding::Value(d)), Some(ty))
                    if d.ty == raw::ValueType::Stored(*ty) => {}
                _ => {
                    return Err(reject(
                        "match generated binding differs from exact active payload",
                    ));
                }
            },
            raw::MatchMode::SharedLoan | raw::MatchMode::ExclusiveLoan => {
                let raw::PlaceOrLoan::Loan(parent) = scrutinee else {
                    return Err(reject("borrowed match requires exact loan source"));
                };
                let access = if *mode == raw::MatchMode::SharedLoan {
                    raw::Access::Shared
                } else {
                    raw::Access::Exclusive
                };
                if f.loans[*parent as usize].access != access {
                    return Err(reject("borrowed match access differs from exact parent"));
                }
                borrowed_binding(f, *parent, access, arm, *payload)?;
            }
        }
    }
    Ok(())
}

fn borrowed_binding(
    f: &raw::Function,
    parent: u32,
    access: raw::Access,
    arm: &raw::EnumArm,
    payload: Option<u32>,
) -> Result<(), Failure> {
    match (&arm.binding, payload) {
        (None, None) => {}
        (Some(raw::MatchBinding::Loan(d)), Some(ty)) => {
            let row = f
                .loans
                .get(d.id as usize)
                .ok_or_else(|| reject("selected binding loan is foreign"))?;
            if d.referent != ty
                || d.access != access
                || row.id != d.id
                || row.referent != d.referent
                || row.access != d.access
                || row.region.role != raw::RegionRole::SelectedMatchArm
                || row.origin
                    != (raw::LoanOrigin::StaticProjection {
                        parent,
                        projection: raw::StaticProjection::EnumPayload(arm.ordinal),
                    })
            {
                return Err(reject(
                    "selected loan differs from exact active referent/access/parent/ordinal",
                ));
            }
        }
        _ => return Err(reject("borrowed match requires only an exact selected loan payload")),
    }
    Ok(())
}
