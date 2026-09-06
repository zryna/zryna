use super::*;

#[test]
fn continued_indexed_source_rejects_immutable_target_and_wrong_exact_rhs() {
    for vector in [false, true] {
        for immutable in [false, true] {
            let (text, raw) =
                structured_owned_fixture::continued_indexed_rejection_fixture(vector, immutable);
            let sources = sources_for(&text);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated continuation descriptor");
            let errors = lower(pair_input(&syntax, &sources)).expect_err("authenticated rejection");
            assert_eq!(
                errors,
                lower(pair_input(&syntax, &sources)).expect_err("authenticated rejection")
            );
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert_eq!(
                errors[0].code(),
                if immutable { "ZRYNA-M3014" } else { "ZRYNA-M3016" },
                "{errors:?}"
            );
            assert_eq!(
                errors[0].message(),
                if immutable {
                    "indexed container is unavailable before its Match operand"
                } else {
                    "owned projection has the wrong exact contextual type"
                }
            );
            assert_eq!(
                errors[0].guidance(),
                if immutable {
                    "retain one complete initialized container with the required access while evaluating its index"
                } else {
                    "use one exact supported Struct field or fixed-array element"
                }
            );
            let needle =
                if immutable { "items[indexValue(offset, \"index\")]" } else { "=> first" };
            let offset = text.find(needle).expect("authenticated continuation descriptor")
                + if immutable { 0 } else { 3 };
            let actual = errors[0].primary_span().expect("authenticated continuation descriptor");
            assert_eq!(
                (actual.start() as usize, actual.end() as usize),
                (offset, offset + if immutable { needle.len() } else { 5 })
            );
            let (text, raw) =
                structured_owned_fixture::continued_indexed_fixture(vector, true, true);
            let sources = sources_for(&text);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated continuation descriptor");
            lower(pair_input(&syntax, &sources)).expect("authenticated continuation descriptor");
        }
    }
}

#[test]
fn continued_indexed_source_bounds_precede_match_without_end_or_reborrow() {
    for vector in [false, true] {
        for owned in [false, true] {
            for write in [false, true] {
                let (text, raw) =
                    structured_owned_fixture::continued_indexed_fixture(vector, owned, write);
                let sources = sources_for(&text);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated indexed continuation");
                let program =
                    lower(pair_input(&syntax, &sources)).expect("exact indexed continuation");
                let replay = lower(pair_input(&syntax, &sources)).expect("replay");
                assert_eq!(
                    format!("{:?}", program.verified_ir()),
                    format!("{:?}", replay.verified_ir())
                );
                let function = program
                    .modules()
                    .next()
                    .expect("authenticated continuation descriptor")
                    .functions()
                    .next()
                    .expect("authenticated continuation descriptor");
                let blocks = function.blocks().collect::<Vec<_>>();
                assert_eq!(blocks.len(), 4);
                let begin = blocks[0]
                    .instructions()
                    .find(|instruction| {
                        instruction.kind() == VerifiedInstructionKind::BeginIndexedAccess
                    })
                    .expect("bounds before Match");
                let initial = begin.borrow().expect("authenticated continuation descriptor");
                assert_index_call(&blocks);
                let first_index =
                    begin.value_operands().next().expect("authenticated continuation descriptor");
                assert_eq!(
                    blocks
                        .iter()
                        .flat_map(|block| block.instructions())
                        .filter(|instruction| instruction.result() == Some(first_index))
                        .count(),
                    1
                );
                assert_eq!(begin.failure_ended_borrows().count(), 0);
                assert_eq!(
                    blocks[0]
                        .terminator()
                        .continued_indexed_accesses()
                        .next()
                        .expect("authenticated continuation descriptor")
                        .borrow(),
                    initial
                );
                for block in &blocks[..3] {
                    assert_eq!(
                        block
                            .terminator()
                            .continued_indexed_accesses()
                            .next()
                            .expect("authenticated continuation descriptor")
                            .borrow(),
                        initial
                    );
                    assert!(block.instructions().all(|instruction| instruction.kind()
                        != VerifiedInstructionKind::EndBorrow
                        || instruction.borrow() != Some(initial)));
                }
                assert_eq!(
                    blocks
                        .iter()
                        .flat_map(|block| block.instructions())
                        .filter(|instruction| instruction.kind()
                            == VerifiedInstructionKind::BeginIndexedAccess)
                        .count(),
                    1
                );
                assert_finish(&blocks, begin, owned, write);
            }
        }
    }
}

fn assert_finish(
    blocks: &[zryna_ir::data_ownership_v1::VerifiedBlock<'_>],
    begin: zryna_ir::data_ownership_v1::VerifiedInstruction<'_>,
    owned: bool,
    write: bool,
) {
    let initial = begin.borrow().expect("initial borrow");
    let finish = blocks[3].instructions().collect::<Vec<_>>();
    let effect = if write {
        if owned {
            VerifiedInstructionKind::BorrowReplace
        } else {
            VerifiedInstructionKind::BorrowWrite
        }
    } else {
        if owned {
            VerifiedInstructionKind::GenericCloneBorrow
        } else {
            VerifiedInstructionKind::BorrowRead
        }
    };
    assert_eq!(finish.iter().filter(|instruction| instruction.kind() == effect).count(), 1);
    let commit = finish
        .iter()
        .find(|instruction| instruction.kind() == effect)
        .expect("authenticated continuation descriptor");
    let final_borrow = commit.borrow().expect("authenticated continuation descriptor");
    assert_eq!(
        finish
            .iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow
                && instruction.borrow() == Some(final_borrow))
            .count(),
        1
    );
    if write {
        let joined =
            blocks[3].parameters().next().expect("authenticated continuation descriptor").id();
        assert_eq!(commit.value_operands().collect::<Vec<_>>(), [joined]);
        assert_eq!(final_borrow, initial);
        if owned {
            for block in &blocks[1..3] {
                let clone = block
                    .instructions()
                    .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
                    .expect("authenticated continuation descriptor");
                assert_eq!(clone.failure_ended_borrows().collect::<Vec<_>>(), [initial]);
                let root =
                    begin.place_operands().next().expect("authenticated continuation descriptor");
                assert!(
                    clone.derived_drop_actions().any(
                        |action| action.root() == root && action.moved_projections().len() == 0
                    )
                );
            }
        }
    }
    if !write {
        let project = finish
            .iter()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::ProjectIndexedBorrow)
            .expect("authenticated continuation descriptor");
        assert_eq!(project.failure_ended_borrows().collect::<Vec<_>>(), [initial]);
        assert_eq!(
            project.indexed_projection().expect("authenticated continuation descriptor").parent(),
            initial
        );
    }
}

fn assert_index_call(blocks: &[zryna_ir::data_ownership_v1::VerifiedBlock<'_>]) {
    let ordered = blocks[0].instructions().collect::<Vec<_>>();
    let call_position = ordered
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("authenticated continuation descriptor");
    let begin_position = ordered
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginIndexedAccess)
        .expect("authenticated continuation descriptor");
    assert!(call_position < begin_position);
    assert_eq!(
        blocks
            .iter()
            .flat_map(|block| block.instructions())
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .count(),
        1
    );
}
