use super::*;
use super::generic_vec_fixture::ordinary_array_composition_fixture::ordinary_array_clone_base_fixture::fixture;
use super::generic_vec_fixture::ordinary_array_composition_fixture::ordinary_array_clone_base_fixture::{Source, sourced_fixture};

#[test]
#[allow(clippy::too_many_lines)]
fn ordinary_array_clone_base_preserves_once_only_materialization_and_bounds_cleanup() {
    for owned in [false, true] {
        for (length, index) in [(2, None), (2, Some(0)), (2, Some(-1)), (2, Some(2)), (0, Some(0))]
        {
            let (source, raw) = fixture(owned, length, index, false);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated cloned array base");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("observe");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let begin_at =
                instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("bounds");
            let begin = instructions[begin_at].indexed_borrow().expect("indexed view");
            assert_eq!(begin.array_length(), Some(u64::from(length)));
            assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
            assert_eq!(instructions[begin_at].kind(), VerifiedInstructionKind::BeginIndexedAccess);
            let calls = instructions
                .iter()
                .enumerate()
                .filter(|(_, i)| i.callee().is_some())
                .collect::<Vec<_>>();
            assert_eq!(calls.len(), usize::from(index.is_none()));
            if let Some((call_at, call)) = calls.first() {
                assert!(*call_at < begin_at);
                assert_eq!(call.result(), Some(begin.index()));
            }
            let end = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("end");
            assert!(begin_at < end);
            if owned {
                assert!(
                    instructions[begin_at + 1..end]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::GenericCloneBorrow)
                );
            } else {
                assert!(
                    instructions[begin_at + 1..end]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::BorrowRead)
                );
            }
            if owned {
                if length != 0 {
                    let base_clones = instructions[..begin_at]
                        .iter()
                        .filter(|i| {
                            matches!(
                                i.kind(),
                                VerifiedInstructionKind::ClonePlace
                                    | VerifiedInstructionKind::GenericClonePlace
                            )
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(base_clones.len(), 1);
                    let source = base_clones[0].place_operands().next().expect("source");
                    let owner = function
                        .places()
                        .find(|place| place.id() == begin.container())
                        .expect("temporary");
                    assert_eq!(
                        owner.kind(),
                        VerifiedPlaceKind::Temporary(base_clones[0].result().expect("cloned base"))
                    );
                    assert_eq!(
                        instructions[begin_at]
                            .derived_drop_actions()
                            .map(|drop| drop.root())
                            .collect::<Vec<_>>(),
                        vec![begin.container(), source]
                    );
                    if let Some((call_at, call)) = calls.first() {
                        assert!(
                            instructions
                                .iter()
                                .position(|i| i.result() == base_clones[0].result())
                                .expect("base clone")
                                < *call_at
                        );
                        assert_eq!(
                            call.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
                            vec![begin.container(), source]
                        );
                    }
                    assert_eq!(
                        function
                            .blocks()
                            .next()
                            .expect("block")
                            .terminator()
                            .derived_drop_actions()
                            .map(|drop| drop.root())
                            .collect::<Vec<_>>(),
                        vec![source]
                    );
                }
                assert_eq!(instructions[end + 1].kind(), VerifiedInstructionKind::DropPlace);
                assert_eq!(
                    instructions[end + 1].place_operands().collect::<Vec<_>>(),
                    vec![begin.container()]
                );
            } else {
                assert!(
                    instructions[..begin_at]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::InitializePlace
                            && i.place_operands().any(|place| place == begin.container()))
                );
            }
        }
    }
}

#[test]
fn ordinary_array_clone_base_static_and_dynamic_subarrays_keep_source_and_temporary_distinct() {
    for owned in [false, true] {
        for source_kind in [Source::Static, Source::Dynamic] {
            let (source, raw) = sourced_fixture(owned, 2, None, false, source_kind);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated subarray clone");
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("observe");
                let instructions =
                    function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                let begins = instructions
                    .iter()
                    .enumerate()
                    .filter_map(|(at, i)| i.indexed_borrow().map(|borrow| (at, borrow)))
                    .collect::<Vec<_>>();
                let expected = if matches!(source_kind, Source::Dynamic) { 2 } else { 1 };
                assert_eq!(begins.len(), expected);
                let (final_at, final_begin) = begins[expected - 1];
                assert_eq!(final_begin.array_length(), Some(2));
                assert_eq!(instructions[final_at].failure_ended_borrows().len(), 0);
                let calls = instructions
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.callee().is_some())
                    .collect::<Vec<_>>();
                assert_eq!(calls.len(), expected);
                assert_eq!(calls.last().expect("index call").1.result(), Some(final_begin.index()));
                if expected == 2 {
                    assert_eq!(calls[0].1.callee().expect("source index").declaration(), 2);
                    assert_eq!(calls[1].1.callee().expect("result index").declaration(), 1);
                    assert!(calls[0].0 < begins[0].0);
                    let first_end = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .expect("source access end");
                    assert!(
                        begins[0].0 < first_end && first_end < calls[1].0 && calls[1].0 < final_at
                    );
                    assert_ne!(begins[0].1.container(), final_begin.container());
                }
                if owned {
                    assert_eq!(instructions[final_at].derived_drop_actions().count(), 2);
                    let observation =
                        instructions[final_at + 1].generic_clone().expect("element clone");
                    assert_eq!(
                        observation.source(),
                        zryna_ir::data_ownership_v1::VerifiedGenericCloneSource::Borrow(
                            final_begin.borrow()
                        )
                    );
                    assert_eq!(
                        instructions[final_at + 1].failure_ended_borrows().collect::<Vec<_>>(),
                        vec![final_begin.borrow()]
                    );
                }
            }
        }
    }
}

#[test]
fn ordinary_array_clone_base_rejects_fresh_mutation_with_exact_replayed_diagnostic() {
    for owned in [false, true] {
        let (source, raw) = fixture(owned, 2, None, true);
        let sources = sources_for(&source);
        let expected = vec![zryna_diagnostics::Diagnostic::error(
            "ZRYNA-Y4002",
            Some("src/main.zry".to_owned()),
            "assignment target is not syntactically a place",
            "return source-faithful canonical protocol-v4 syntax",
        )];
        for _ in 0..2 {
            assert_eq!(
                verify_snapshot(raw.clone(), &sources).expect_err("fresh mutation"),
                expected
            );
        }
    }
}
