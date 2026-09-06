use super::super::super::preparation_state::Checkpoint;
use super::{Consumption, Operation, Span, Step, Ty, raw};
use crate::data_ownership_v1::owner_state::OwnerDelta;
use crate::data_ownership_v1::scalar_operations::ScalarOperation;

impl Consumption<'_, '_, '_, '_> {
    pub(super) fn execute(
        &mut self,
        index: usize,
        length: usize,
        step: Step<'_>,
        vec_actions: Option<usize>,
    ) -> Option<raw::ValueId> {
        self.require_next(&step.operation);
        let mut effects = Vec::new();
        let emission = match step.operation {
            Operation::DropTemporary { place } => {
                effects = self.drop_temporary(place, step.at);
                None
            }
            Operation::StructuredValue { expression, value } => {
                Some(self.lowerer.consume_structured_handoff(expression, value, step.ty))
            }
            Operation::ReplaceProjection { place, value } => {
                effects = self.replace_projection(place, value, step.ty, step.at);
                None
            }
            Operation::VecPush { vector, value, cleanup } => {
                effects = self.vec_push(vector, value, cleanup, step.ty, step.at);
                None
            }
            Operation::IndexedCopyStorage { place, value } => {
                self.indexed_copy_storage(place, value, step.ty, step.at);
                None
            }
            operation @ (Operation::IndexedEnter { .. }
            | Operation::IndexedExit
            | Operation::IndexedEffect(_)) => {
                effects = self.indexed_step(index, operation, step.at);
                None
            }
            Operation::ScalarEnter { kind, end, operands } => {
                self.enter_scalar((index, end, length), step.ty, kind, operands);
                None
            }
            Operation::ScalarCommit { kind, operands } => {
                Some(self.scalar_commit(index, step.ty, step.at, kind, &operands))
            }
            Operation::CallEnter { signature, end, arguments } => {
                self.enter_call(
                    (index, end, length),
                    signature,
                    arguments,
                    (step.ty, step.at),
                    vec_actions.expect("call cleanup demand"),
                );
                None
            }
            Operation::CallTransfer { value, owner } => {
                effects.push(self.transfer_call(index, value, owner, step.ty));
                None
            }
            Operation::CallRelease => {
                self.release_call(index, step.ty);
                None
            }
            Operation::CallCommit { signature, arguments, cleanup } => {
                Some(self.commit_call(index, signature, arguments, cleanup, (step.ty, step.at)))
            }
            operation @ (Operation::StringEnter { .. }
            | Operation::StringRead(_)
            | Operation::StringExit) => {
                self.string_step(index, length, step.ty, operation);
                None
            }
            Operation::Enter { arity, kind, end } => {
                self.enter(index, length, (step.ty, step.at), (arity, kind, end), vec_actions);
                None
            }
            Operation::Release => {
                self.release(index, step.ty);
                None
            }
            Operation::Prefix { id, descriptor } => {
                assert!(self.cleanups.is_empty(), "projection cannot interrupt cleanup effects");
                self.lowerer.consume_prepared_prefix(id, descriptor);
                None
            }
            Operation::CloneCapacity { aggregate } => {
                self.clone_capacity(aggregate, step.at);
                None
            }
            Operation::Cleanup { id, actions, prefix } => {
                self.prepared_cleanup(id, actions, prefix, step.at);
                None
            }
            Operation::GenericClonePrefix { id, owner, actions } => {
                self.generic_clone_prefix(id, owner, actions, step.at);
                None
            }
            Operation::Leaf(leaf) => Some(self.consume_leaf(index, leaf, step.ty, step.at)),
            operation @ (Operation::Commit { .. } | Operation::VecCommit { .. }) => {
                Some(self.commit_step(index, (step.ty, step.at), operation))
            }
        };
        let value = emission.as_ref().map(|emission| emission.value);
        let owners =
            emission.as_ref().map_or(effects.as_slice(), |emission| emission.owners.as_slice());
        assert_eq!(value, step.value, "prepared value identity");
        assert_eq!(owners, step.owners, "prepared ordered owner effects");
        self.finish_step(index, value, step.ty, step.after);
        value
    }

    fn finish_step(
        &mut self,
        index: usize,
        value: Option<raw::ValueId>,
        ty: Ty,
        after: Checkpoint,
    ) {
        if let Some(value) = value {
            self.record_result(index, value, ty);
        }
        assert_eq!(self.lowerer.preparation_checkpoint(), after, "prepared step effects");
    }

    fn drop_temporary(&mut self, place: raw::PlaceId, at: Span) -> Vec<OwnerDelta> {
        self.lowerer.emit_prepared_effect(at, raw::InstructionKind::DropPlace { place });
        let effect = self.lowerer.owners.consume_owner(place).expect("prepared temporary owner");
        self.lowerer.preparation_facts.apply(effect);
        vec![effect]
    }

    fn enter_scalar(
        &mut self,
        range: (usize, usize, usize),
        ty: Ty,
        kind: ScalarOperation,
        operands: Vec<(raw::ValueId, Ty)>,
    ) {
        assert!(self.cleanups.is_empty(), "scalar entry cannot interrupt cleanup");
        self.scalars.enter(range, self.open.len(), ty, kind, operands);
    }
}
