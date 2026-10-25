//! Checked static place and nonstored referent projection.
use super::{
    Diagnostic, LoanAddress, Record, Shape, allocate, budget, invariant, reserve, shape, take,
};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedFunction, VerifiedLoan, VerifiedLoanOrigin, VerifiedPlace, raw,
};
pub(super) fn places(
    f: VerifiedFunction<'_, '_>,
    values: &[Record],
    parameter_values: &[Option<u32>],
    types: &[Shape<'_>],
    end: &mut u64,
    work: &mut u64,
) -> Result<Vec<Record>, Diagnostic> {
    let mut places = reserve(f.places().len())?;
    let mut place_cache = reserve(f.places().len())?;
    place_cache.resize(f.places().len(), None);
    let mut place_views = reserve(f.places().len())?;
    place_views.extend(f.places());
    for p in &place_views {
        places.push(
            Places {
                views: &place_views,
                cache: &mut place_cache,
                values,
                params: parameter_values,
                types,
                end,
                work,
            }
            .resolve(*p, 0)?,
        );
    }
    Ok(places)
}
mod loans;
pub(super) use loans::derive as loans;

struct Places<'c, 'p, 'a, 't> {
    views: &'c [VerifiedPlace<'p, 'a>],
    cache: &'c mut [Option<Record>],
    values: &'c [Record],
    params: &'c [Option<u32>],
    types: &'c [Shape<'t>],
    end: &'c mut u64,
    work: &'c mut u64,
}
impl Places<'_, '_, '_, '_> {
    fn resolve(&mut self, p: VerifiedPlace<'_, '_>, depth: usize) -> Result<Record, Diagnostic> {
        take(self.work, 1)?;
        if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
            return Err(budget());
        }
        if let Some(r) = self.cache[p.id() as usize] {
            return Ok(r);
        }
        let result = match p.kind() {
            raw::PlaceKind::Temporary(id) => *self.values.get(id as usize).ok_or_else(invariant)?,
            raw::PlaceKind::Parameter(ordinal) => *self
                .values
                .get(self.params.get(ordinal as usize).copied().flatten().ok_or_else(invariant)?
                    as usize)
                .ok_or_else(invariant)?,
            raw::PlaceKind::Local(_) => {
                allocate(self.types, raw::ValueType::Stored(p.ty()), self.end)?
            }
            raw::PlaceKind::StructField { base, ordinal }
            | raw::PlaceKind::EnumPayload { base, variant: ordinal }
            | raw::PlaceKind::FixedArrayConstant { base, index: ordinal } => {
                let parent =
                    self.resolve(*self.views.get(base as usize).ok_or_else(invariant)?, depth + 1)?;
                let projection = match p.kind() {
                    raw::PlaceKind::StructField { .. } => {
                        raw::StaticProjection::StructField(ordinal)
                    }
                    raw::PlaceKind::EnumPayload { .. } => {
                        raw::StaticProjection::EnumPayload(ordinal)
                    }
                    _ => raw::StaticProjection::FixedArrayConstant(ordinal),
                };
                let offset = displacement(self.types, parent.ty, projection, p.ty(), self.work)?;
                Record {
                    offset: parent.offset.checked_add(offset).ok_or_else(budget)?,
                    ty: raw::ValueType::Stored(p.ty()),
                }
            }
        };
        if result.ty != raw::ValueType::Stored(p.ty()) {
            return Err(invariant());
        }
        self.cache[p.id() as usize] = Some(result);
        Ok(result)
    }
}

fn displacement(
    types: &[Shape<'_>],
    parent: raw::ValueType,
    projection: raw::StaticProjection,
    child: u32,
    work: &mut u64,
) -> Result<u32, Diagnostic> {
    let raw::ValueType::Stored(parent) = parent else {
        return Err(invariant());
    };
    let parent = shape(types, parent)?.view;
    take(
        work,
        (parent.fields().len().max(parent.variants().len()) as u64)
            .checked_mul(16)
            .and_then(|n| n.checked_add(16))
            .ok_or_else(budget)?,
    )?;
    let (ty, offset) = match projection {
        raw::StaticProjection::StructField(ordinal) => {
            let (_, ty, offset) = parent.fields().nth(ordinal as usize).ok_or_else(invariant)?;
            (ty.index(), offset)
        }
        raw::StaticProjection::EnumPayload(ordinal) => {
            let (_, ty) = parent.variants().nth(ordinal as usize).ok_or_else(invariant)?;
            (ty.ok_or_else(invariant)?.index(), parent.payload().ok_or_else(invariant)?.0)
        }
        raw::StaticProjection::FixedArrayConstant(index) => {
            if u64::from(index) >= parent.array_length().ok_or_else(invariant)? {
                return Err(invariant());
            }
            (
                parent.referenced_type().ok_or_else(invariant)?.index(),
                u64::from(index)
                    .checked_mul(parent.array_stride().ok_or_else(invariant)?)
                    .ok_or_else(budget)?,
            )
        }
    };
    if ty != child
        || offset.checked_add(shape(types, child)?.view.size()).is_none_or(|n| n > parent.size())
    {
        return Err(invariant());
    }
    u32::try_from(offset).map_err(|_| budget())
}
