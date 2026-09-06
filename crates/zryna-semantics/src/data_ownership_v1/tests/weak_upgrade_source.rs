use super::generic_vec_fixture::weak_upgrade_fixture::{Case, fixture, fixture_case};
use super::*;
use zryna_diagnostics::Diagnostic;

#[test]
fn weak_upgrade_source_seals_success_only_owner_and_retains_addressable_operand() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated weak upgrade");
    let program = lower(pair_input(&syntax, &sources)).expect("weak upgrade lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    let upgrade = blocks[0].terminator();
    assert_eq!(upgrade.kind(), VerifiedTerminatorKind::WeakUpgradeBranch);
    assert_eq!(upgrade.place_operands().count(), 1);
    let (success, expired) = upgrade.weak_upgrade_edges().expect("upgrade outcomes");
    assert_ne!(success.target(), expired.target());
    let success_block =
        blocks.iter().find(|block| block.id() == success.target()).expect("success block");
    let expired_block =
        blocks.iter().find(|block| block.id() == expired.target()).expect("expired block");
    assert_eq!(success_block.parameters().count(), 1);
    assert_eq!(expired_block.parameters().count(), 0);
    assert!(upgrade.derived_drop_actions().count() >= 2);
}

#[test]
fn temporary_weak_upgrade_operand_is_prepared_once_and_released_on_both_outcomes() {
    let (source, raw) = fixture(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated temporary weak upgrade");
    let program = lower(pair_input(&syntax, &sources)).expect("temporary weak upgrade lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(
        blocks[0]
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
            .count(),
        1
    );
    let upgrade = blocks[0].terminator();
    let operand = upgrade.place_operands().next().expect("temporary operand");
    let clone = blocks[0]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakClone)
        .expect("single weak clone");
    let source_weak = clone.place_operands().next().expect("retained Weak source");
    let shared = blocks[0]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::WeakDowngrade)
        .expect("source downgrade")
        .place_operands()
        .next()
        .expect("retained Shared");
    assert_ne!(operand, source_weak, "producer owns one distinct temporary");
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source_weak, shared],
        "producer failure retains both sources and excludes its uncommitted result"
    );
    assert_eq!(
        upgrade.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [operand, source_weak, shared],
        "overflow releases completed temporary before preceding roots"
    );
    let (success, expired) = upgrade.weak_upgrade_edges().expect("upgrade outcomes");
    for target in [success.target(), expired.target()] {
        let block = blocks.iter().find(|block| block.id() == target).expect("outcome block");
        let first = block.instructions().next().expect("temporary release before body");
        assert_eq!(first.kind(), VerifiedInstructionKind::DropPlace);
        assert_eq!(first.place_operands().collect::<Vec<_>>(), [operand]);
        assert_eq!(
            block
                .instructions()
                .filter(|instruction| {
                    instruction.kind() == VerifiedInstructionKind::DropPlace
                        && instruction.place_operands().next() == Some(operand)
                })
                .count(),
            1
        );
        let expected =
            if target == success.target() { vec![source_weak, shared] } else { vec![source_weak] };
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root())
                .collect::<Vec<_>>(),
            expected,
            "normal outcome cleanup excludes released temporary and returned handle"
        );
    }
    assert!(upgrade.derived_drop_actions().any(|action| action.root() == operand));
}

#[test]
fn weak_upgrade_source_diagnostics_are_exact_deterministic_and_recover() {
    for (case, code, message, guidance, selected) in [
        (
            Case::WrongType,
            "ZRYNA-M3013",
            "weak upgrade operand does not have one exact Weak type",
            "upgrade one available Weak<T> handle",
            "owner",
        ),
        (
            Case::Missing,
            "ZRYNA-M3002",
            "aggregate value 'ghost' is not declared",
            "reference one exact preceding local using its declared spelling",
            "ghost",
        ),
        (
            Case::ExpiredBindingUse,
            "ZRYNA-M3002",
            "aggregate value 'upgraded' is not declared",
            "reference one exact preceding local using its declared spelling",
            "upgraded",
        ),
    ] {
        let (source, raw) = fixture_case(case);
        let body = &raw.files[0].functions[0].body;
        let expression = if matches!(case, Case::ExpiredBindingUse) {
            let RawStatementKind::Return { value, .. } = body.statements[4].kind else {
                panic!("expired return");
            };
            value
        } else {
            let RawStatementKind::WeakUpgrade { weak, .. } = body.statements[2].kind else {
                panic!("upgrade statement");
            };
            weak
        };
        let zryna_syntax::v4::RawExpressionKind::Reference { ref name } =
            body.expressions[expression as usize].kind
        else {
            panic!("selected reference");
        };
        assert_eq!(name.text, selected);
        let at = name.span;
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected upgrade");
        let expected = vec![Diagnostic::error_at(code, span(&sources, at), message, guidance)];
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("upgrade rejection"),
                expected
            );
        }
    }
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery upgrade");
    lower(pair_input(&syntax, &sources)).expect("valid lowering after rejections");
}
