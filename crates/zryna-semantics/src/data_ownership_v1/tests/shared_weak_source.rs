use super::generic_vec_fixture::shared_weak_fixture::{Case, fixture, fixture_case};
use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn shared_and_weak_source_operations_preserve_exact_handle_ownership() {
    let (source, raw) = fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated shared/weak fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("shared/weak source lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let operations = instructions
        .iter()
        .filter_map(|instruction| match instruction.kind() {
            VerifiedInstructionKind::SharedConstruct
            | VerifiedInstructionKind::SharedClone
            | VerifiedInstructionKind::WeakDowngrade
            | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
        ]
    );
    assert_eq!(
        function.blocks().next().expect("block").terminator().derived_drop_actions().count(),
        3,
        "returned Shared is excluded while cloned Shared and both Weak handles release"
    );
}

#[test]
fn shared_and_weak_source_lowering_replays_identically() {
    let (source, raw) = fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated shared/weak fixture");
    let first = format!("{:?}", lower(pair_input(&syntax, &sources)).expect("first lowering"));
    let second = format!("{:?}", lower(pair_input(&syntax, &sources)).expect("replay lowering"));
    assert_eq!(first, second);
}

#[test]
fn shared_and_weak_scalar_payload_categories_lower_through_verified_ir() {
    for case in [Case::ScalarBool, Case::ScalarI32] {
        let (source, raw) = fixture_case(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated scalar handle fixture");
        let program = lower(pair_input(&syntax, &sources)).expect("scalar handle lowering");
        let kinds = program
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .filter_map(|instruction| match instruction.kind() {
                VerifiedInstructionKind::SharedConstruct
                | VerifiedInstructionKind::SharedClone
                | VerifiedInstructionKind::WeakDowngrade
                | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                VerifiedInstructionKind::SharedConstruct,
                VerifiedInstructionKind::SharedClone,
                VerifiedInstructionKind::WeakDowngrade,
                VerifiedInstructionKind::WeakClone,
            ]
        );
    }
}

#[test]
fn nested_shared_payload_moves_into_outer_control_without_implicit_clone() {
    let (source, raw) = fixture_case(Case::NestedShared);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated nested handle fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("nested handle lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let kinds = block
        .instructions()
        .filter_map(|instruction| match instruction.kind() {
            VerifiedInstructionKind::SharedConstruct
            | VerifiedInstructionKind::SharedClone
            | VerifiedInstructionKind::WeakDowngrade
            | VerifiedInstructionKind::WeakClone => Some(instruction.kind()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
        ]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 3);
}

fn rejected(case: Case, message: &str, guidance: &str) {
    let (source, raw) = fixture_case(case);
    let expression = raw.files[0].functions[0]
        .body
        .expressions
        .iter()
        .rev()
        .find(|expression| matches!(expression.kind, RawExpressionKind::Clone { .. }))
        .expect("rejected clone expression");
    let at = expression.span;
    assert!(source[at.start as usize..at.end as usize].starts_with("clone("));
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected handle source");
    let expected = vec![Diagnostic::error_at(
        if matches!(case, Case::MovedReuse) { "ZRYNA-M3014" } else { "ZRYNA-M3013" },
        span(&sources, at),
        message,
        guidance,
    )];
    for _ in 0..2 {
        assert_eq!(lower(pair_input(&syntax, &sources)).expect_err("handle rejection"), expected);
    }
}

#[test]
fn moved_handle_clone_reports_exact_diagnostic_and_replays() {
    rejected(
        Case::MovedReuse,
        "shared or weak handle is moved or unavailable",
        "use one complete initialized handle before moving it",
    );
}

#[test]
fn wrong_handle_clone_type_reports_exact_diagnostic_and_replays() {
    rejected(
        Case::WrongCloneType,
        "shared or weak operation has the wrong exact handle type",
        "clone one exact handle or downgrade Shared<T> to Weak<T>",
    );
}
