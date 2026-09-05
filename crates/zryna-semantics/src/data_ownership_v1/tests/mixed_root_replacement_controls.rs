use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::tests::nested_mixed_construction::root_replacement::{
    ReplacementCase, ReplacementRoot, replacement_fixture,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawStatementKind;

fn commit_size(root: ReplacementRoot) -> usize {
    let (source, snapshot) = replacement_fixture(root, ReplacementCase::Constructor);
    let mut count = 0;
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        let assignment = lowerer
            .function
            .body
            .statements
            .iter()
            .position(|statement| matches!(statement.kind, RawStatementKind::Assignment { .. }))
            .expect("assignment");
        for index in 0..=assignment {
            assert!(run_statement(lowerer, index, ty));
        }
        count = lowerer.instructions.len();
    });
    assert!(errors.is_empty(), "{errors:?}");
    count
}

fn capacity(root: ReplacementRoot, extra: bool, invalid: bool) {
    let total = commit_size(root);
    let case = if invalid { ReplacementCase::InvalidLater } else { ReplacementCase::Constructor };
    let (source, snapshot) = replacement_fixture(root, case);
    let held = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - total + usize::from(extra);
    let mut expected = None;
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        let assignment = lowerer
            .function
            .body
            .statements
            .iter()
            .position(|statement| matches!(statement.kind, RawStatementKind::Assignment { .. }))
            .expect("assignment");
        for index in 0..assignment {
            assert!(run_statement(lowerer, index, ty));
        }
        let assignment_at = lowerer.function.body.statements[assignment].span;
        for _ in 0..held {
            lowerer.credit_ledger().acquire_assignment();
        }
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        let succeeded = run_statement(lowerer, assignment, ty);
        assert_eq!(succeeded, !(extra || invalid));
        if invalid || extra {
            assert_eq!(
                state(lowerer),
                before,
                "rejected replacement preserves complete prior state"
            );
            assert_eq!(lowerer.preparation_facts, facts);
        } else {
            assert_eq!(
                lowerer.instructions.len() + held,
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            );
            assert_eq!(lowerer.instructions.len(), total);
            assert!(matches!(
                lowerer.instructions.last().expect("commit").kind,
                raw::InstructionKind::ReplacePlace { .. }
            ));
        }
        if invalid {
            let expression = lowerer.function.body.expressions.iter().find(|expression| {
                matches!(&expression.kind, zryna_syntax::v4::RawExpressionKind::Reference { name } if name.text == "lost")
            }).expect("unresolved later RHS child");
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3002",
                crate::data_ownership_v1::span(lowerer.input.sources(), expression.span),
                "aggregate value 'lost' is not declared",
                "reference one exact preceding local using its declared spelling",
            ));
        } else if extra {
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3201",
                crate::data_ownership_v1::span(lowerer.input.sources(), assignment_at),
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            ));
        }
        for _ in 0..held {
            lowerer.credit_ledger().release_assignment();
        }
        assert_eq!(lowerer.reserved_transitions, 0);
        assert!(lowerer.constructor_storage_is_clear());
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn mixed_root_replacement_final_transition_exact_and_first_extra_preserve_state() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for extra in [false, true] {
            capacity(root, extra, false);
        }
    }
}

#[test]
fn mixed_root_replacement_later_semantic_error_precedes_commit_budget_without_mutation() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for extra in [false, true] {
            capacity(root, extra, true);
        }
    }
}

#[test]
fn mixed_root_replacement_invalid_targets_and_self_moves_preserve_prior_statements() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for case in [
            ReplacementCase::Immutable,
            ReplacementCase::Moved,
            ReplacementCase::WrongType,
            ReplacementCase::SelfDirect,
        ] {
            let (source, snapshot) = replacement_fixture(root, case);
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                let assignment = lowerer
                    .function
                    .body
                    .statements
                    .iter()
                    .position(|statement| {
                        matches!(statement.kind, RawStatementKind::Assignment { .. })
                    })
                    .expect("assignment");
                for index in 0..assignment {
                    assert!(run_statement(lowerer, index, ty));
                }
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                assert!(!run_statement(lowerer, assignment, ty));
                assert_eq!(state(lowerer), before, "{root:?} {case:?}");
                assert_eq!(lowerer.preparation_facts, facts);
            });
            assert_eq!(errors.len(), 1, "{root:?} {case:?}: {errors:?}");
        }
    }
}

#[test]
fn mixed_root_replacement_self_retention_precedes_exhausted_rhs_capacity() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        let (source, snapshot) = replacement_fixture(root, ReplacementCase::SelfDirect);
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let RawStatementKind::Assignment { value, .. } =
                lowerer.function.body.statements[1].kind
            else {
                panic!("self assignment")
            };
            let at = lowerer.function.body.expressions[value as usize].span;
            let held = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
            for _ in 0..held {
                lowerer.credit_ledger().acquire_assignment();
            }
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            assert!(!run_statement(lowerer, 1, ty));
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3014",
                crate::data_ownership_v1::span(lowerer.input.sources(), at),
                "owned aggregate assignment cannot consume its destination while preparing its replacement",
                "clone the destination or prepare a distinct aggregate value before replacement",
            ));
            for _ in 0..held {
                lowerer.credit_ledger().release_assignment();
            }
            assert_eq!(lowerer.reserved_transitions, 0);
        });
        assert_eq!(errors, [expected.expect("destination diagnostic")]);
    }
}
