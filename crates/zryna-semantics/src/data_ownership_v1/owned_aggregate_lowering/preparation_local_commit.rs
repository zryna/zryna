use super::super::super::Binding;
use super::super::super::owner_state::OwnerDelta;
use super::super::preparation_state::Checkpoint;
use super::{PreparedValue, PrivateOwnedAggregateLowerer, Span, Ty, raw};

pub(in crate::data_ownership_v1::owned_aggregate_lowering) struct PreparedLocal<'l, 'a, 'f, 'e> {
    value: PreparedValue<'l, 'a, 'f, 'e>,
    after: Checkpoint,
    place: raw::PlaceId,
    local: u32,
    next_local: u32,
    ty: Ty,
    at: Span,
    name: String,
    mutable: bool,
    owner: Option<OwnerDelta>,
}

impl<'l, 'a, 'f, 'e> PreparedLocal<'l, 'a, 'f, 'e> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn prepare(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        ty: Ty,
        at: Span,
        name: &str,
        mutable: bool,
    ) -> Option<Self> {
        assert_eq!(
            lowerer.local_preparation_route(ty),
            super::super::mixed_shape::PreparationRoute::MixedSummary
        );
        let value = PreparedValue::prepare_local(lowerer, id, ty)?;
        let after = value.plan.steps.last()?.after;
        let usage = super::resource_replay::usage(after);
        if !usage.local_place(at, value.lowerer.errors)
            || !usage.transition(1, at, value.lowerer.errors)
        {
            return None;
        }
        let place = raw::PlaceId(u32::try_from(after.counts[1]).ok()?);
        let local = value.lowerer.next_local;
        let next_local = local.checked_add(1)?;
        let owner = if ty.is_copy() {
            None
        } else {
            Some(value.plan.owners.rename_effect(value.plan.result, place)?.1)
        };
        assert!(!value.lowerer.bindings.contains_key(name), "prepared local name is fresh");
        Some(Self {
            value,
            after,
            place,
            local,
            next_local,
            ty,
            at,
            name: name.to_owned(),
            mutable,
            owner,
        })
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn consume(self) -> raw::PlaceId {
        let Self { value, after, place, local, next_local, ty, at, name, mutable, owner } = self;
        let PreparedValue { lowerer, plan } = value;
        assert_eq!(lowerer.next_local, local, "prepared local identity");
        let result = PreparedValue { lowerer: &mut *lowerer, plan }.consume();
        assert_eq!(
            lowerer.preparation_checkpoint(),
            after,
            "prepared local initializer checkpoint"
        );
        assert_eq!(lowerer.places.len(), place.0 as usize, "prepared dense local place");
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(local),
        });
        lowerer.next_local = next_local;
        lowerer.emit_prepared_effect(
            at,
            raw::InstructionKind::InitializePlace { place, value: result },
        );
        if let Some(expected) = owner {
            let actual = lowerer.owners.rename(result, place).expect("prepared local owner rename");
            assert_eq!(actual, expected, "prepared local exact owner delta");
            lowerer.preparation_facts.apply(actual);
        }
        assert!(
            lowerer.bindings.insert(name, Binding { ty, place, mutable }).is_none(),
            "prepared local binding installed once"
        );
        let mut committed = after;
        committed.counts[1] += 1;
        committed.counts[2] += 1;
        assert_eq!(lowerer.preparation_checkpoint(), committed, "prepared local final checkpoint");
        assert_eq!(lowerer.next_local, next_local, "prepared local next identity");
        place
    }
}
