use super::{Consumption, Span, Ty, raw};
use crate::data_ownership_v1::owner_state::OwnerDelta;

impl Consumption<'_, '_, '_, '_> {
    pub(super) fn replace_projection(
        &mut self,
        place: raw::PlaceId,
        value: raw::ValueId,
        ty: Ty,
        at: Span,
    ) -> Vec<OwnerDelta> {
        self.lowerer.emit_prepared_effect(
            at,
            if ty.is_copy() {
                raw::InstructionKind::ReplacePlace { place, value }
            } else {
                raw::InstructionKind::GenericReplacePlace { place, value }
            },
        );
        let mut effects = Vec::new();
        if !ty.is_copy() {
            let delta =
                self.lowerer.owners.transfer(value).expect("prepared static replacement owner");
            self.lowerer.preparation_facts.apply(delta);
            effects.push(delta);
        }
        effects
    }

    pub(super) fn vec_push(
        &mut self,
        vector: raw::PlaceId,
        value: raw::ValueId,
        cleanup: raw::CleanupPlanId,
        ty: Ty,
        at: Span,
    ) -> Vec<OwnerDelta> {
        assert_eq!(self.cleanups, [(cleanup, None)], "push exact preparation cleanup");
        self.cleanups.clear();
        self.lowerer
            .emit_prepared_effect(at, raw::InstructionKind::VecPush { vector, value, cleanup });
        let mut effects = Vec::new();
        if !ty.is_copy() {
            let delta = self.lowerer.owners.transfer(value).expect("prepared push argument owner");
            self.lowerer.preparation_facts.apply(delta);
            effects.push(delta);
        }
        effects
    }
}
