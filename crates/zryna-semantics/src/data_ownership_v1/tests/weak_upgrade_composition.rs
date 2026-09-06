use super::generic_vec_fixture::weak_upgrade_fixture::composition_fixture::{Context, fixture};
use super::*;

#[path = "weak_upgrade_composition_cleanup.rs"]
mod cleanup;

#[test]
fn weak_upgrade_composition_nested_repeated_control_and_operand_paths_are_verified() {
    for (context, count) in [
        (Context::Nested, 2),
        (Context::Repeated, 2),
        (Context::Conditional, 4),
        (Context::Loop, 3),
        (Context::Call, 1),
        (Context::Match, 1),
    ] {
        let (source, raw) = fixture(context);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources)
            .unwrap_or_else(|errors| panic!("{context:?}: {source}\n{errors:?}"));
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{context:?}: {source}\n{errors:?}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("entry function");
        let blocks = function.blocks().collect::<Vec<_>>();
        cleanup::assert_exact(function, context);
        let upgrades = blocks
            .iter()
            .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::WeakUpgradeBranch)
            .collect::<Vec<_>>();
        assert_eq!(upgrades.len(), count, "{context:?}");
        for block in upgrades {
            let terminator = block.terminator();
            let (success, expired) = terminator.weak_upgrade_edges().expect("upgrade outcomes");
            let yes =
                blocks.iter().find(|block| block.id() == success.target()).expect("success block");
            let no =
                blocks.iter().find(|block| block.id() == expired.target()).expect("expired block");
            assert_eq!(yes.parameters().count(), 1);
            assert_eq!(
                yes.parameters().next().expect("synthesized owner").ty(),
                function.result_type()
            );
            assert_eq!(no.parameters().count(), 0);
            let operand = terminator.place_operands().next().expect("retained Weak");
            assert_eq!(
                terminator.derived_drop_actions().filter(|action| action.root() == operand).count(),
                1
            );
        }
        if matches!(context, Context::Call) {
            assert_eq!(
                blocks
                    .iter()
                    .flat_map(|block| block.instructions())
                    .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
                    .count(),
                1
            );
        }
        if matches!(context, Context::Match) {
            assert_eq!(
                blocks
                    .iter()
                    .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::EnumMatch)
                    .count(),
                1
            );
        }
        assert_eq!(
            format!("{program:?}"),
            format!(
                "{:?}",
                lower(pair_input(&syntax, &sources)).expect("exact composition replay")
            )
        );
    }
}

#[test]
fn weak_upgrade_composition_match_inference_rejects_wrong_arm_and_nonuniform_types() {
    for context in [Context::WrongArm, Context::Nonuniform] {
        let (source, raw) = fixture(context);
        let sources = sources_for(&source);
        let expression = raw.files[0].functions[0]
            .body
            .expressions
            .iter()
            .find(|expression| {
                matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
            })
            .expect("operand Match");
        let (at, code, message, guidance) = if matches!(context, Context::WrongArm) {
            let zryna_syntax::v4::RawExpressionKind::Match { ref arms, .. } = expression.kind
            else {
                unreachable!()
            };
            (
                arms[1].span,
                "ZRYNA-M3009",
                "match arm repeats or names a foreign variant",
                "provide each exact declared variant once",
            )
        } else {
            (
                expression.span,
                "ZRYNA-M3013",
                "weak upgrade operand does not produce an exact handle type",
                "produce one exact Weak<T> handle before upgrading it",
            )
        };
        let expected = vec![zryna_diagnostics::Diagnostic::error_at(
            code,
            span(&sources, at),
            message,
            guidance,
        )];
        let syntax = verify_snapshot(raw, &sources).expect("authenticated hostile Match operand");
        for _ in 0..2 {
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("type inference rejects"),
                expected,
                "{context:?}: {source}"
            );
        }
    }
    let (source, raw) = fixture(Context::Match);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery Match");
    let first = lower(pair_input(&syntax, &sources)).expect("valid Match recovery");
    assert_eq!(
        format!("{first:?}"),
        format!("{:?}", lower(pair_input(&syntax, &sources)).expect("exact recovery replay"))
    );
}
