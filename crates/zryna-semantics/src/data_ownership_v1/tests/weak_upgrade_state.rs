use super::generic_vec_fixture::weak_upgrade_fixture::{Case, fixture_case};
use super::*;

#[test]
fn weak_upgrade_state_rejections_are_authenticated_and_replay() {
    for (case, code, message, guidance) in [
        (
            Case::Moved,
            "ZRYNA-M3014",
            "weak upgrade handle is moved or unavailable",
            "upgrade one complete initialized Weak handle",
        ),
        (
            Case::Reused,
            "ZRYNA-M3014",
            "aggregate value 'weak' is moved or only partially available",
            "move a whole owned aggregate only before moving any of its projections",
        ),
        (
            Case::ActiveBorrow,
            "ZRYNA-M3017",
            "borrow authority cannot cross a structured ownership edge",
            "end lexical borrows before a branch, loop edge, or continuation",
        ),
        (
            Case::UnequalJoin,
            "ZRYNA-M3015",
            "structured ownership edges have unequal definite states",
            "restore the same live owners and initialization masks without implicit repair",
        ),
    ] {
        let (source, raw) = fixture_case(case);
        let sources = sources_for(&source);
        let body = &raw.files[0].functions[0].body;
        let upgrade = body
            .statements
            .iter()
            .find(|statement| matches!(statement.kind, RawStatementKind::WeakUpgrade { .. }))
            .expect("upgrade statement");
        let selected = match case {
            Case::Moved => {
                let RawStatementKind::WeakUpgrade { weak, .. } = upgrade.kind else {
                    unreachable!()
                };
                body.expressions[weak as usize].span
            }
            Case::Reused => {
                let RawStatementKind::LocalDeclaration { initializer, .. } =
                    body.statements[3].kind
                else {
                    panic!("reuse declaration")
                };
                body.expressions[initializer as usize].span
            }
            _ => upgrade.span,
        };
        let syntax =
            verify_snapshot(raw, &sources).unwrap_or_else(|errors| panic!("{source}\n{errors:?}"));
        let first = lower(pair_input(&syntax, &sources)).expect_err("invalid upgrade state");
        assert_eq!(
            first,
            vec![zryna_diagnostics::Diagnostic::error_at(
                code,
                span(&sources, selected),
                message,
                guidance
            )],
            "{source}"
        );
        assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("same rejection"));
    }
    let (source, raw) = fixture_case(Case::RetainedJoin);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated equal join");
    let program = lower(pair_input(&syntax, &sources)).expect("valid state recovery");
    assert_eq!(
        format!("{program:?}"),
        format!("{:?}", lower(pair_input(&syntax, &sources)).expect("pristine replay"))
    );
}

#[test]
fn weak_upgrade_state_retains_exact_weak_root_through_join_and_later_cleanup() {
    let (source, raw) = fixture_case(Case::RetainedJoin);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated retained weak");
    let program = lower(pair_input(&syntax, &sources)).expect("retained weak join");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    let upgrade = blocks[0].terminator();
    let weak = upgrade.place_operands().next().expect("exact Weak root");
    let shared = blocks[0]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakDowngrade)
        .expect("downgrade")
        .place_operands()
        .next()
        .expect("retained Shared source");
    assert_eq!(
        upgrade.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [weak, shared]
    );
    assert!(!blocks[0].instructions().any(|instruction| matches!(
        instruction.kind(),
        VerifiedInstructionKind::MoveFromPlace | VerifiedInstructionKind::DropPlace
    )
        && instruction.place_operands().any(|place| place == weak)));
    assert_eq!(blocks.len(), 4);
    let (success, expired) = upgrade.weak_upgrade_edges().expect("success and expiration");
    let yes = blocks.iter().find(|block| block.id() == success.target()).expect("success");
    let no = blocks.iter().find(|block| block.id() == expired.target()).expect("expired");
    assert_eq!(
        yes.instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
            .count(),
        1,
        "only synthesized success binding is released"
    );
    assert!(
        !yes.instructions()
            .chain(no.instructions())
            .any(|instruction| instruction.place_operands().any(|place| place == weak))
    );
    assert_eq!(
        blocks[3]
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [weak],
        "later return releases retained Weak exactly once, excluding returned Shared"
    );
}
