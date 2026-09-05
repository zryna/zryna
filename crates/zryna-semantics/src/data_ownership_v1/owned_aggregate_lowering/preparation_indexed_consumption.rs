use super::{Consumption, Operation, Span, raw};
use crate::data_ownership_v1::owner_state::OwnerDelta;

#[derive(Default)]
pub(super) struct IndexedScopes(Vec<IndexedScope>);

struct IndexedScope {
    start: usize,
    end: usize,
    result: usize,
    depth: usize,
}

#[test]
fn indexed_result_routing_respects_nested_expression_consumers() {
    let mut scopes = IndexedScopes(vec![IndexedScope { start: 2, end: 12, result: 10, depth: 0 }]);
    assert!(scopes.outward(4, 0, Some(3)), "nested scalar/call receives its operand");
    assert!(!scopes.outward(5, 0, Some(1)), "completed index is not an outer operand");
    assert!(scopes.outward(10, 0, Some(1)), "selected element is an outer operand");
    assert!(scopes.outward(4, 1, None), "nested constructor records its own children");
    scopes.0.push(IndexedScope { start: 5, end: 9, result: 8, depth: 0 });
    assert!(!scopes.outward(6, 0, Some(3)), "inner index is hidden from enclosing scalar");
    assert!(scopes.outward(8, 0, Some(3)), "inner selected value reaches enclosing scalar");
    assert!(!scopes.outward(8, 0, Some(1)), "outer index scope still hides inner results");
}

impl IndexedScopes {
    pub(super) fn outward(&self, index: usize, depth: usize, consumer: Option<usize>) -> bool {
        self.0.iter().all(|scope| {
            scope.depth != depth || consumer > Some(scope.start) || scope.result == index
        })
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
                self.indexed.0.push(IndexedScope {
                    start: index,
                    end,
                    result,
                    depth: self.open.len(),
                });
            }
            Operation::IndexedExit => {
                let scope = self.indexed.0.pop().expect("indexed scope");
                assert_eq!(
                    (scope.end, scope.depth),
                    (index, self.open.len()),
                    "indexed scope exact end"
                );
            }
            Operation::IndexedEffect(kind) => {
                match &kind {
                    raw::InstructionKind::BeginIndexedBorrow { definition, cleanup, .. }
                    | raw::InstructionKind::BeginIndexedAccess { definition, cleanup, .. } => {
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
                    raw::InstructionKind::BindIndexedBorrow { parent, borrow } => {
                        assert!(self.cleanups.is_empty(), "binding is infallible");
                        let facts = &mut self.lowerer.preparation_facts;
                        assert_eq!(borrow.0, facts.next_borrow);
                        facts.next_borrow += 1;
                        let authority =
                            facts.active_borrows.remove(parent).expect("transient parent");
                        assert!(facts.active_borrows.insert(*borrow, authority).is_none());
                    }
                    raw::InstructionKind::ProjectIndexedBorrow {
                        parent, borrow, cleanup, ..
                    } => {
                        assert_eq!(self.cleanups, [(*cleanup, None)], "projected bounds cleanup");
                        self.cleanups.clear();
                        let facts = &mut self.lowerer.preparation_facts;
                        assert_eq!(borrow.0, facts.next_borrow);
                        facts.next_borrow += 1;
                        let authority =
                            facts.active_borrows.remove(parent).expect("transient parent");
                        assert!(facts.active_borrows.insert(*borrow, authority).is_none());
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
                    raw::InstructionKind::DropPlace { place } => {
                        let delta = self
                            .lowerer
                            .owners
                            .consume_owner(*place)
                            .expect("fresh indexed base owner");
                        self.lowerer.preparation_facts.apply(delta);
                        effects.push(delta);
                    }
                    _ => unreachable!("indexed effect vocabulary"),
                }
                self.lowerer.emit_prepared_effect(at, kind);
            }
            _ => unreachable!("indexed operation"),
        }
        effects
    }
}
