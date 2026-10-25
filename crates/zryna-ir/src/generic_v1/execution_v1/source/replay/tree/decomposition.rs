//! One deterministic direct sibling inventory for each touched closed Struct/FixedArray path.
use super::{Body, Failure, Kind, UntrustedSpan, charge, exhausted, raw, reject, reserve};
impl Body<'_, '_> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn static_child(
        &mut self,
        base: u32,
        ty: u32,
        projection: raw::StaticProjection,
        span: UntrustedSpan,
    ) -> Result<u32, Failure> {
        self.expand_children(base, span)?;
        let kind = self.world.kind(self.output.places[base as usize].ty);
        let ((Some(Kind::Struct { .. }), raw::StaticProjection::StructField(ordinal))
        | (Some(Kind::FixedArray { .. }), raw::StaticProjection::FixedArrayConstant(ordinal))) =
            (kind, projection)
        else {
            return Err(reject("source projection kind differs from exact static parent"));
        };
        charge(self.spent, 1)?;
        let id = *self.children[base as usize]
            .get(ordinal as usize)
            .ok_or_else(|| reject("source static child ordinal absent"))?;
        if self.output.places[id as usize].ty != ty {
            return Err(Failure::InternalFailure);
        }
        Ok(id)
    }
    pub(super) fn expand_children(
        &mut self,
        base: u32,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        if self.expanded[base as usize] {
            return Ok(());
        }
        let ty = self.output.places[base as usize].ty;
        let kind = self.world.kind(ty).ok_or(Failure::InternalFailure)?;
        let count = match kind {
            Kind::Struct { .. } => self.world.members(ty).ok_or(Failure::InternalFailure)?.len(),
            Kind::FixedArray { length } => {
                usize::try_from(length).map_err(|_| Failure::InternalFailure)?
            }
            _ => return Err(reject("source split needs exact Struct/FixedArray decomposition")),
        };
        let end = self.output.places.len().checked_add(count).ok_or(Failure::InternalFailure)?;
        if end > crate::data_ownership_v1::MAX_PLACES_PER_FUNCTION {
            return Err(exhausted(
                "complete source sibling inventory exceeds inherited place limit",
            ));
        }
        charge(
            self.spent,
            count.checked_mul(size_of::<u32>() + 2).ok_or(Failure::InternalFailure)?,
        )?;
        let mut children = reserve(count)?;
        for index in 0..count {
            let ordinal = u32::try_from(index).map_err(|_| Failure::InternalFailure)?;
            let (child, place) = match kind {
                Kind::Struct { .. } => (
                    self.world
                        .members(ty)
                        .and_then(|m| m[index])
                        .ok_or(Failure::InternalFailure)?,
                    raw::PlaceKind::StructField { base, ordinal },
                ),
                Kind::FixedArray { .. } => (
                    self.world
                        .arguments(ty)
                        .and_then(|a| a.first())
                        .copied()
                        .ok_or(Failure::InternalFailure)?,
                    raw::PlaceKind::FixedArrayConstant { base, index: ordinal },
                ),
                _ => return Err(Failure::InternalFailure),
            };
            children.push(self.place(child, place, span)?);
        }
        self.children[base as usize] = children;
        self.expanded[base as usize] = true;
        Ok(())
    }
}
