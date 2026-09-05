use super::generic_static_fixture::{Case, Shape, fixture};
use super::*;

#[test]
fn generic_static_source_subtree_clone_and_move_keep_exact_parent_masks() {
    for shape in [Shape::Struct, Shape::Array, Shape::ArrayStruct] {
        for case in [Case::Clone, Case::Move] {
            let (source, raw) = fixture(shape, case);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated static subtree");
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{shape:?} {case:?}: {errors:?}\n{source}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let operation = block
                    .instructions()
                    .find(|instruction| {
                        instruction.place_operands().any(|id| {
                            function.places().any(|p| {
                                p.id() == id
                                    && matches!(
                                        p.kind(),
                                        VerifiedPlaceKind::StructField { .. }
                                            | VerifiedPlaceKind::FixedArrayConstant { .. }
                                    )
                            })
                        })
                    })
                    .expect("static operation");
                let projection = operation.place_operands().next().expect("subtree");
                let place = function.places().find(|p| p.id() == projection).expect("projection");
                let parent = match place.kind() {
                    VerifiedPlaceKind::StructField { base, ordinal: 0 }
                    | VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } => base,
                    other => panic!("unexpected subtree: {other:?}"),
                };
                let cleanup = block
                    .terminator()
                    .derived_drop_actions()
                    .find(|a| a.root() == parent)
                    .expect("retained enclosing owner");
                if matches!(case, Case::Move) {
                    assert_eq!(operation.kind(), VerifiedInstructionKind::GenericMoveFromPlace);
                    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [projection]);
                } else {
                    let clone = operation.generic_clone().expect("structural projected clone");
                    assert_eq!(clone.ty(), place.ty());
                    assert_ne!(clone.destination(), projection);
                    assert_eq!(cleanup.moved_projections().len(), 0);
                    assert!(
                        operation
                            .derived_drop_actions()
                            .any(|a| a.root() == parent && a.moved_projections().len() == 0)
                    );
                    assert_eq!(
                        operation
                            .generic_clone_prefix_failure_drop_actions()
                            .next()
                            .expect("recursive prefix")
                            .root(),
                        clone.destination()
                    );
                }
            }
        }
    }
}

#[test]
fn generic_static_source_sibling_clone_retains_moved_subtree_mask_on_failure() {
    for shape in [Shape::Array, Shape::ArrayStruct] {
        let (source, raw) = fixture(shape, Case::Sibling);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated disjoint subtree");
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{shape:?}: {errors:?}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let moved = function
            .places()
            .find(|p| matches!(p.kind(), VerifiedPlaceKind::FixedArrayConstant { index: 0, .. }))
            .expect("moved sibling");
        let retained = function
            .places()
            .find(|p| matches!(p.kind(), VerifiedPlaceKind::FixedArrayConstant { index: 1, .. }))
            .expect("retained sibling");
        let VerifiedPlaceKind::FixedArrayConstant { base, .. } = moved.kind() else {
            unreachable!()
        };
        let clone =
            block.instructions().find(|i| i.generic_clone().is_some()).expect("sibling clone");
        assert_eq!(clone.place_operands().collect::<Vec<_>>(), [retained.id()]);
        for actions in [
            clone.derived_drop_actions().collect::<Vec<_>>(),
            block.terminator().derived_drop_actions().collect(),
        ] {
            let drop = actions.iter().find(|a| a.root() == base).expect("partial enclosing owner");
            assert_eq!(drop.moved_projections().collect::<Vec<_>>(), [moved.id()]);
            assert!(!drop.moved_projections().any(|p| p == retained.id()));
        }
    }
}

#[test]
fn generic_static_source_repeated_and_self_clone_replacements_retain_target_until_commit() {
    for shape in [Shape::Struct, Shape::Array, Shape::ArrayStruct] {
        for case in [Case::Replace, Case::Repeat, Case::SelfClone] {
            let (source, raw) = fixture(shape, case);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated static replacement");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{shape:?} {case:?}: {errors:?}\n{source}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let commits = instructions
                .iter()
                .enumerate()
                .filter(|(_, i)| i.kind() == VerifiedInstructionKind::GenericReplacePlace)
                .collect::<Vec<_>>();
            assert_eq!(commits.len(), if matches!(case, Case::Repeat) { 2 } else { 1 });
            for (position, commit) in commits {
                let target = commit.place_operands().next().expect("target");
                let place = function.places().find(|p| p.id() == target).expect("target place");
                let parent = match place.kind() {
                    VerifiedPlaceKind::StructField { base, .. }
                    | VerifiedPlaceKind::FixedArrayConstant { base, .. } => base,
                    other => panic!("replacement must address subtree: {other:?}"),
                };
                let rhs = commit.value_operands().next().expect("prepared RHS");
                let preparation = instructions[..position]
                    .iter()
                    .find(|i| i.result() == Some(rhs))
                    .expect("RHS before commit");
                assert!(
                    preparation
                        .derived_drop_actions()
                        .any(|a| a.root() == parent && a.moved_projections().len() == 0)
                );
                assert_eq!(
                    commit.derived_drop_actions().map(|a| a.root()).collect::<Vec<_>>(),
                    [target]
                );
                assert!(!block.terminator().derived_drop_actions().any(|a| a.root() == parent));
            }
        }
    }
}

#[test]
fn generic_static_source_partial_root_and_self_consuming_rhs_are_stable_rejections() {
    for shape in [Shape::Struct, Shape::Array, Shape::ArrayStruct] {
        for case in [Case::Partial, Case::SelfMove] {
            let (source, raw) = fixture(shape, case);
            let body = &raw.files[0].functions[0].body;
            let (at, message, guidance) = if matches!(case, Case::Partial) {
                let at = body.expressions.iter().rfind(|e| matches!(&e.kind, zryna_syntax::v4::RawExpressionKind::Reference { name } if name.text == "item")).expect("root clone reference").span;
                (
                    at,
                    "aggregate value 'item' is moved or only partially available",
                    "clone the aggregate only before moving any owned projection",
                )
            } else {
                let at = body
                    .statements
                    .iter()
                    .find(|s| matches!(s.kind, RawStatementKind::Assignment { .. }))
                    .expect("self replacement")
                    .span;
                (
                    at,
                    "static replacement target is immutable, unavailable, or consumed during preparation",
                    "retain the exact complete mutable subobject until replacement commits",
                )
            };
            let sources = sources_for(&source);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated rejected static source");
            let expected = vec![zryna_diagnostics::Diagnostic::error_at(
                "ZRYNA-M3014",
                span(&sources, at),
                message,
                guidance,
            )];
            for _ in 0..2 {
                assert_eq!(
                    lower(pair_input(&syntax, &sources)).expect_err("ownership rejection"),
                    expected,
                    "{shape:?} {case:?}: {source}"
                );
            }
        }
    }
}
