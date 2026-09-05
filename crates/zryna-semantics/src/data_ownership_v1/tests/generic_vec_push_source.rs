use super::generic_vec_fixture::{Element, Operation, fixture};
use super::*;

#[test]
fn generic_vec_push_source_prepares_owned_elements_before_growth_and_retains_exact_failure_owners()
{
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        for operation in [Operation::Push, Operation::PushClone, Operation::PushIndexedClone] {
            let (source, raw) = fixture(&element, operation, None);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated generic Vec push");
            let mut previous = None;
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{element:?} {operation:?}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let pushes = instructions
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind() == VerifiedInstructionKind::VecPush)
                    .collect::<Vec<_>>();
                assert_eq!(pushes.len(), 1);
                let (push_at, push) = pushes[0];
                let target = push.place_operands().next().expect("push target");
                let rhs = push.value_operands().next().expect("owned element");
                let produced = instructions
                    .iter()
                    .position(|i| i.result() == Some(rhs))
                    .expect("prepared RHS");
                assert!(produced < push_at);
                let owner = function
                    .places()
                    .find(|p| p.kind() == VerifiedPlaceKind::Temporary(rhs))
                    .expect("prepared owner");
                let growth = push.derived_drop_actions().collect::<Vec<_>>();
                assert_eq!(growth[0].root(), owner.id());
                assert!(growth.iter().any(|drop| drop.root() == target));
                assert!(growth.iter().all(|drop| drop.moved_projections().count() == 0));
                assert!(
                    block
                        .terminator()
                        .derived_drop_actions()
                        .all(|drop| drop.root() != owner.id() && drop.root() != target)
                );
                if matches!(operation, Operation::PushClone | Operation::PushIndexedClone) {
                    assert!(instructions[produced].cleanup().is_some());
                    assert!(
                        instructions[produced]
                            .derived_drop_actions()
                            .any(|drop| drop.root() == target)
                    );
                }
                if matches!(operation, Operation::PushIndexedClone) {
                    let end = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .expect("read authority ends before growth");
                    assert!(produced < end && end < push_at);
                }
                let trace = (instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(), growth);
                if let Some(previous) = previous.replace(trace.clone()) {
                    assert_eq!(previous, trace);
                }
            }
        }
    }
}

#[test]
fn generic_vec_push_source_copy_values_have_no_rhs_drop_obligation() {
    for element in [Element::I32, Element::Bool] {
        let (source, raw) = fixture(&element, Operation::Push, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated Copy push");
        let program = lower(pair_input(&syntax, &sources)).expect("Copy push");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let push = function
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .find(|i| i.kind() == VerifiedInstructionKind::VecPush)
            .expect("push");
        let target = push.place_operands().next().expect("target");
        assert_eq!(
            push.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
            [target]
        );
    }
}

#[test]
fn generic_vec_push_source_immutable_target_rejects_at_complete_operation_before_rhs() {
    for element in [Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let (mut source, raw) = fixture(&element, Operation::PushClone, None);
        let RawStatementKind::LocalDeclaration { keyword_span, .. } =
            raw.files[0].functions[0].body.statements[0].kind
        else {
            panic!("items local");
        };
        source.replace_range(keyword_span.start as usize..keyword_span.end as usize, "const");
        let mut raw = shift_snapshot(raw, keyword_span.end, 2);
        let RawStatementKind::LocalDeclaration { mutable, .. } =
            &mut raw.files[0].functions[0].body.statements[0].kind
        else {
            panic!("local");
        };
        *mutable = false;
        let body = &raw.files[0].functions[0].body;
        let push = body
            .expressions
            .iter()
            .find(|e| matches!(e.kind, zryna_syntax::v4::RawExpressionKind::VecPush { .. }))
            .expect("push")
            .span;
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated immutable target");
        let expected = vec![zryna_diagnostics::Diagnostic::error_at(
            "ZRYNA-M3014",
            span(&sources, push),
            "push target is immutable, unavailable, or consumed during value preparation",
            "retain one complete mutable Vec until its prepared element is appended",
        )];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("immutable Vec push"),
                expected
            );
        }
    }
}
