//! Exact static-place geometry, checked arithmetic and original per-projection work charges.
use super::{Diagnostic, add, align, budget, charge, invariant, raw};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedFunction, VerifiedParameter,
    raw::{PlaceKind, ValueType},
};
use zryna_layout::generic_v1::TypeView;
pub(super) struct Geometry<'b, 'a> {
    pub(super) values: &'b [raw::Slot],
    pub(super) places: &'b [raw::Slot],
    pub(super) types: &'b [TypeView<'a>],
}
pub(super) fn place_offset(
    f: VerifiedFunction<'_, '_>,
    p: zryna_ir::generic_v1::execution_v1::VerifiedPlace<'_, '_>,
    s: &raw::Slot,
    geometry: &Geometry<'_, '_>,
    cursor: &mut u32,
    work: &mut usize,
) -> Result<u32, Diagnostic> {
    let values = geometry.values;
    let places = geometry.places;
    let types = geometry.types;
    let result = match p.kind() {
        PlaceKind::Parameter(ordinal) => {
            charge(work, ordinal as usize + 1)?;
            let VerifiedParameter::Value(v) =
                f.parameters().nth(ordinal as usize).ok_or_else(invariant)?
            else {
                return Err(invariant());
            };
            values[v.id() as usize].offset
        }
        PlaceKind::Temporary(id) => values.get(id as usize).ok_or_else(invariant)?.offset,
        PlaceKind::Local(_) => {
            *cursor = align(*cursor, s.alignment)?;
            let start = *cursor;
            *cursor = add(*cursor, s.bytes)?;
            start
        }
        PlaceKind::StructField { base, ordinal } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            charge(work, ordinal as usize + 1)?;
            add(
                parent.offset,
                u32::try_from(
                    types[ty as usize].fields().nth(ordinal as usize).ok_or_else(invariant)?.2,
                )
                .map_err(|_| budget())?,
            )?
        }
        PlaceKind::EnumPayload { base, .. } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            add(
                parent.offset,
                u32::try_from(types[ty as usize].payload().ok_or_else(invariant)?.0)
                    .map_err(|_| budget())?,
            )?
        }
        PlaceKind::FixedArrayConstant { base, index } => {
            let parent = places.get(base as usize).ok_or_else(invariant)?;
            let ValueType::Stored(ty) = parent.ty else { return Err(invariant()) };
            let offset = types[ty as usize]
                .array_stride()
                .ok_or_else(invariant)?
                .checked_mul(u64::from(index))
                .ok_or_else(budget)?;
            add(parent.offset, u32::try_from(offset).map_err(|_| budget())?)?
        }
    };
    Ok(result)
}
