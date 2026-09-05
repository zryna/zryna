use super::{Consumption, Operation, Span, raw};
use crate::data_ownership_v1::owner_state::OwnerDelta;

#[derive(Default)]
pub(super) struct IndexedScopes(Vec<(usize, usize, usize)>);

impl IndexedScopes {
    pub(super) fn outward(&self, index: usize, depth: usize) -> bool {
        self.0.iter().all(|(_, result, parent)| *parent != depth || *result == index)
    }
    pub(super) fn complete(&self) -> bool {
        self.0.is_empty()
    }
}

impl Consumption<'_, '_, '_, '_> {
    pub(super) fn indexed_step(
        &mut self,
        index: usize,
        operation: Operation<'_>,
        at: Span,
    ) -> Vec<OwnerDelta> {
        let mut effects = Vec::new();
        match operation {
            Operation::IndexedEnter { end, result } => {
                assert!(index < result && result < end, "indexed result lies inside scope");
                self.indexed.0.push((end, result, self.open.len()));
            }
            Operation::IndexedExit => {
                let (end, _, depth) = self.indexed.0.pop().expect("indexed scope");
                assert_eq!((end, depth), (index, self.open.len()), "indexed scope exact end");
            }
            Operation::IndexedEffect(kind) => {
                match &kind {
                    raw::InstructionKind::BeginIndexedBorrow { definition, cleanup, .. } => {
                        assert_eq!(self.cleanups, [(*cleanup, None)], "indexed bounds cleanup");
                        self.cleanups.clear();
                        assert_eq!(definition.id.0, self.lowerer.preparation_facts.next_borrow);
                        self.lowerer.preparation_facts.next_borrow += 1;
                        self.lowerer
                            .preparation_facts
                            .active_borrows
                            .insert(definition.id, (definition.place, definition.access));
                    }
                    raw::InstructionKind::EndBorrow { borrow } => {
                        assert!(
                            self.lowerer.preparation_facts.active_borrows.remove(borrow).is_some()
                        );
                    }
                    raw::InstructionKind::BeginBorrow(definition) => {
                        assert_eq!(definition.id.0, self.lowerer.preparation_facts.next_borrow);
                        self.lowerer.preparation_facts.next_borrow += 1;
                        self.lowerer
                            .preparation_facts
                            .active_borrows
                            .insert(definition.id, (definition.place, definition.access));
                    }
                    raw::InstructionKind::BorrowReplace { value, .. } => {
                        let delta = self
                            .lowerer
                            .owners
                            .transfer(*value)
                            .expect("prepared indexed replacement");
                        self.lowerer.preparation_facts.apply(delta);
                        effects.push(delta);
                    }
                    raw::InstructionKind::BorrowWrite { .. } => {}
                    _ => unreachable!("indexed effect vocabulary"),
                }
                self.lowerer.emit_prepared_effect(at, kind);
            }
            _ => unreachable!("indexed operation"),
        }
        effects
    }
}
