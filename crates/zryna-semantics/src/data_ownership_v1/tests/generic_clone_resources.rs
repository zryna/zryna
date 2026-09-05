use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::tests::generic_clone_source::fixture;
use crate::data_ownership_v1::tests::nested_mixed_construction::root_replacement::ReplacementRoot;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::{RawExpressionKind, RawStatementKind};

fn boundary(root: ReplacementRoot, extra: bool, missing: bool) {
    let (mut source, mut snapshot) = fixture(root, false);
    if missing {
        let body = &mut snapshot.files[0].functions[0].body;
        let input = body
            .expressions
            .iter()
            .find_map(|expression| match expression.kind {
                RawExpressionKind::Clone { value, .. } => Some(value),
                _ => None,
            })
            .expect("clone input");
        let expression = &mut body.expressions[input as usize];
        let RawExpressionKind::Reference { name } = &mut expression.kind else {
            panic!("reference");
        };
        source.replace_range(name.span.start as usize..name.span.end as usize, "lost");
        name.text = "lost".into();
    }
    let mut expected = None;
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        let returned = lowerer
            .function
            .body
            .statements
            .iter()
            .position(|statement| matches!(statement.kind, RawStatementKind::Return { .. }))
            .expect("return");
        for index in 0..returned {
            assert!(run_statement(lowerer, index, ty));
        }
        let RawStatementKind::Return { value, .. } =
            lowerer.function.body.statements[returned].kind
        else {
            panic!("return");
        };
        let clone_at = lowerer.function.body.expressions[value as usize].span;
        let pending = lowerer.owners.pending().to_vec();
        let required = 2 * pending.len() + 1;
        let held = ir::MAX_DROP_ACTIONS_PER_FUNCTION - lowerer.cleanup_actions - required
            + usize::from(extra);
        lowerer.preparation_facts.held_cleanup[1] = held;
        let before = state(lowerer);
        let checkpoint = lowerer.preparation_checkpoint();
        let facts = lowerer.preparation_facts.clone();
        if extra || missing {
            assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
            assert_eq!(lowerer.preparation_facts, facts);
            let (code, at, message, guidance) = if missing {
                let RawExpressionKind::Clone { value, .. } =
                    lowerer.function.body.expressions[value as usize].kind
                else {
                    panic!("clone");
                };
                (
                    "ZRYNA-M3002",
                    lowerer.function.body.expressions[value as usize].span,
                    "aggregate binding 'lost' is not declared in this function",
                    "clone one preceding available aggregate local",
                )
            } else {
                (
                    "ZRYNA-M3201",
                    clone_at,
                    "structural clone exceeds a checked value, place, or cleanup resource limit",
                    "reduce simultaneously live owned aggregates or clone sites",
                )
            };
            expected = Some(Diagnostic::error_at(
                code,
                crate::data_ownership_v1::span(lowerer.input.sources(), at),
                message,
                guidance,
            ));
        } else {
            let prepared =
                PreparedValue::prepare(lowerer, value, ty).expect("exact cleanup action limit");
            assert_eq!(state(prepared.lowerer), before);
            assert_eq!(prepared.lowerer.preparation_checkpoint(), checkpoint);
            prepared.consume();
            assert_eq!(lowerer.cleanup_actions + held, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
            assert_eq!(lowerer.preparation_facts.held_cleanup[1], held);
            assert_eq!(&lowerer.owners.pending()[..pending.len()], pending.as_slice());
            assert_eq!(lowerer.owners.pending().len(), pending.len() + 1);
        }
        lowerer.preparation_facts.held_cleanup[1] = 0;
        assert!(lowerer.constructor_storage_is_clear());
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn generic_clone_resource_exact_first_extra_preserve_source_state_credits_and_recovery() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        boundary(root, false, false);
        boundary(root, true, false);
        boundary(root, false, false);
    }
}

#[test]
fn generic_clone_missing_source_precedes_deferred_cleanup_capacity() {
    for extra in [false, true] {
        boundary(ReplacementRoot::Struct, extra, true);
    }
}
