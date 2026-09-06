use super::super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::super::constructor_resources::tests::{run_statement, with_snapshot};
use crate::data_ownership_v1::tests::finite_recursive_fixture::{SOURCE, snapshot};
use zryna_ir::data_ownership_v1::{self as ir, raw};

#[derive(Clone, Copy, Debug)]
enum Boundary {
    Clone,
    Push,
}

impl Boundary {
    const fn statement(self) -> usize {
        match self {
            Self::Clone => 6,
            Self::Push => 8,
        }
    }

    const fn demand(self, pending: usize) -> usize {
        match self {
            Self::Clone => 2 * pending + 1,
            Self::Push => 3 * pending + 2,
        }
    }

    const fn diagnostic_spelling(self, overflow: bool) -> &'static str {
        match (self, overflow) {
            (Self::Push, true) => "clone(copy)",
            (Self::Clone, _) => "clone(target)",
            (Self::Push, false) => "push(forest, clone(copy))",
        }
    }
}

fn exercise(boundary: Boundary, extra: bool, overflow: bool) -> Vec<zryna_diagnostics::Diagnostic> {
    with_snapshot(SOURCE, snapshot(), |lowerer, ty| {
        for statement in 0..boundary.statement() {
            assert!(run_statement(lowerer, statement, ty));
        }
        let pending = lowerer.owners.pending().len();
        let held = if overflow {
            usize::MAX
        } else {
            ir::MAX_DROP_ACTIONS_PER_FUNCTION - lowerer.cleanup_actions - boundary.demand(pending)
                + usize::from(extra)
        };
        lowerer.preparation_facts.held_cleanup[1] = held;
        let before = state(lowerer);
        let checkpoint = lowerer.preparation_checkpoint();
        let facts = lowerer.preparation_facts.clone();
        let instruction_start = lowerer.instructions.len();
        let succeeded = run_statement(lowerer, boundary.statement(), ty);
        assert_eq!(succeeded, !extra && !overflow);
        if succeeded {
            assert_eq!(lowerer.cleanup_actions + held, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
        } else {
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
            assert_eq!(lowerer.preparation_facts, facts);
        }
        lowerer.preparation_facts.held_cleanup[1] = 0;
        if !succeeded {
            assert!(run_statement(lowerer, boundary.statement(), ty));
            assert!(lowerer.moved_projections.is_empty());
            assert!(lowerer.partial_roots.is_empty());
            let recovered = &lowerer.instructions[instruction_start..];
            let clone = recovered
                .iter()
                .find(|instruction| {
                    matches!(instruction.kind, raw::InstructionKind::GenericClonePlace { .. })
                })
                .expect("recovered recursive clone");
            let result = clone.result.as_ref().expect("clone result").id;
            let destination = lowerer
                .places
                .iter()
                .find(|place| place.kind == raw::PlaceKind::Temporary(result))
                .expect("exact prepared owner")
                .id;
            if matches!(boundary, Boundary::Push) {
                let push = recovered
                    .iter()
                    .find(|instruction| {
                        matches!(instruction.kind, raw::InstructionKind::VecPush { .. })
                    })
                    .expect("recovered push");
                let raw::InstructionKind::VecPush { vector, value, cleanup } = push.kind else {
                    unreachable!("VecPush selected")
                };
                assert_eq!(value, result);
                let actions = &lowerer.cleanup_plans[cleanup.0 as usize].actions;
                assert_eq!(actions[0], raw::DropAction::DropPlace(destination));
                assert_eq!(actions[1], raw::DropAction::DropPlace(vector));
            }
        }
        assert!(lowerer.constructor_storage_is_clear());
    })
}

#[test]
fn finite_recursive_clone_and_push_resources_are_exact_atomic_and_recoverable() {
    for boundary in [Boundary::Clone, Boundary::Push] {
        assert!(exercise(boundary, false, false).is_empty());
        for (extra, overflow) in [(true, false), (false, true)] {
            let diagnostics = exercise(boundary, extra, overflow);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            let (message, guidance) = if extra && matches!(boundary, Boundary::Push) {
                (
                    "derived cleanup actions exceed the per-function M3 limit of 262144",
                    "reduce simultaneously live owned aggregates and String leaves",
                )
            } else {
                (
                    "structural clone exceeds a checked value, place, or cleanup resource limit",
                    "reduce simultaneously live owned aggregates or clone sites",
                )
            };
            assert_eq!(
                diagnostics[0].message(),
                message,
                "{boundary:?} extra={extra} overflow={overflow}"
            );
            assert_eq!(diagnostics[0].guidance(), guidance);
            let spelling = boundary.diagnostic_spelling(overflow);
            let expected_start = u32::try_from(SOURCE.find(spelling).expect("clone spelling"))
                .expect("fixture offset");
            let span = diagnostics[0].primary_span().expect("source diagnostic");
            assert_eq!(
                (span.start(), span.end()),
                (
                    expected_start,
                    expected_start + u32::try_from(spelling.len()).expect("fixture length"),
                )
            );
            let stable = |diagnostic: &zryna_diagnostics::Diagnostic| {
                let span = diagnostic.primary_span().expect("source diagnostic");
                (
                    diagnostic.code().to_owned(),
                    span.start(),
                    span.end(),
                    diagnostic.message().to_owned(),
                    diagnostic.guidance().to_owned(),
                )
            };
            let replay = exercise(boundary, extra, overflow);
            assert_eq!(stable(&diagnostics[0]), stable(&replay[0]), "stable diagnostic replay");
        }
        assert!(exercise(boundary, false, false).is_empty(), "recovery after rejection");
    }
}
