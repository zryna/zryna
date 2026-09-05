use super::super::super::owner_state::{OwnerDelta, OwnerState};
use super::super::preparation_plan::PreparationFacts;
use super::super::preparation_state::Checkpoint;
use super::{PreparedValue, PrivateOwnedAggregateLowerer, Span, Ty, raw};

pub(in crate::data_ownership_v1::owned_aggregate_lowering) struct PreparedReplacement<
    'l,
    'a,
    'f,
    'e,
> {
    value: PreparedValue<'l, 'a, 'f, 'e>,
    after: Checkpoint,
    target: raw::PlaceId,
    at: Span,
    owner: OwnerDelta,
    owners: OwnerState,
    facts: PreparationFacts,
}

impl<'l, 'a, 'f, 'e> PreparedReplacement<'l, 'a, 'f, 'e> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn prepare(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        ty: Ty,
        target: raw::PlaceId,
        at: Span,
    ) -> Option<Self> {
        assert!(lowerer.whole_root_available(target), "available mixed replacement target");
        let place = lowerer.places.get(target.0 as usize).expect("mixed replacement place");
        assert_eq!(place.ty, ty.ir, "exact mixed replacement target type");
        assert!(matches!(place.kind, raw::PlaceKind::Local(_)), "mixed replacement local root");
        let value = PreparedValue::prepare_replacement(lowerer, id, ty, target)?;
        let after = value.plan.steps.last()?.after;
        if !super::resource_replay::usage(after).transition(1, at, value.lowerer.errors) {
            return None;
        }
        let mut owners = value.plan.owners.clone();
        let owner = owners.replace(value.plan.result, target).expect("prepared replacement owner");
        let mut facts = value.plan.facts.clone();
        facts.apply(owner);
        Some(Self { value, after, target, at, owner, owners, facts })
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn consume(self) {
        let Self { value, after, target, at, owner, owners, facts } = self;
        let PreparedValue { lowerer, plan } = value;
        let result = PreparedValue { lowerer: &mut *lowerer, plan }.consume();
        assert_eq!(lowerer.preparation_checkpoint(), after, "prepared replacement RHS checkpoint");
        assert!(lowerer.whole_root_available(target), "replacement retains complete destination");
        lowerer.emit_prepared_effect(
            at,
            raw::InstructionKind::ReplacePlace { place: target, value: result },
        );
        let actual = lowerer.owners.replace(result, target).expect("prepared replacement commit");
        assert_eq!(actual, owner, "prepared replacement exact owner delta");
        lowerer.preparation_facts.apply(actual);
        assert_eq!(lowerer.owners, owners, "prepared replacement final owners");
        assert_eq!(lowerer.preparation_facts, facts, "prepared replacement final facts");
        let mut committed = after;
        committed.counts[2] += 1;
        committed.pending -= 1;
        assert_eq!(
            lowerer.preparation_checkpoint(),
            committed,
            "prepared replacement final checkpoint"
        );
    }
}
