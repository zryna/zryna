use super::*;

fn assert_fresh_access_flow(
    instructions: &[zryna_ir::data_ownership_v1::VerifiedInstruction<'_>],
    vector: bool,
    owned: bool,
) {
    let call = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("index producer");
    let begin = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginIndexedAccess)
        .expect("single bounds operation");
    let finish = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("single final end");
    assert!(call < begin && begin < finish);
    assert_eq!(
        instructions
            .iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginIndexedAccess)
            .count(),
        1
    );
    assert_eq!(
        instructions
            .iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
            .count(),
        1,
        "fresh Match storage is never ended and reborrowed"
    );
    let access = instructions[begin].indexed_borrow().expect("indexed authority");
    assert_eq!(instructions[finish].borrow(), Some(access.borrow()));
    assert_eq!(instructions[call].result(), Some(access.index()));
    assert_fresh_base_lifetime(
        instructions,
        [call, begin, finish],
        access.container(),
        vector,
        owned,
    );
    let observation = if owned {
        VerifiedInstructionKind::GenericCloneBorrow
    } else {
        VerifiedInstructionKind::BorrowRead
    };
    let observed = instructions
        .iter()
        .find(|instruction| instruction.kind() == observation)
        .expect("exact final observation");
    assert_eq!(observed.borrow(), Some(access.borrow()));
    if owned {
        assert_eq!(observed.failure_ended_borrows().collect::<Vec<_>>(), [access.borrow()]);
        assert!(observed.derived_drop_actions().any(|action| action.root() == access.container()));
    }
}

fn assert_fresh_base_lifetime(
    instructions: &[zryna_ir::data_ownership_v1::VerifiedInstruction<'_>],
    positions: [usize; 3],
    container: zryna_ir::data_ownership_v1::PlaceIdentity,
    vector: bool,
    owned: bool,
) {
    let [call, begin, finish] = positions;
    if vector || owned {
        for index in [call, begin] {
            assert!(
                instructions[index].derived_drop_actions().any(|action| action.root() == container),
                "fresh base survives preparation failure"
            );
        }
        assert_eq!(instructions[finish + 1].kind(), VerifiedInstructionKind::DropPlace);
        assert_eq!(instructions[finish + 1].place_operands().collect::<Vec<_>>(), [container]);
    } else {
        assert_eq!(
            instructions
                .iter()
                .filter(|instruction| {
                    instruction.kind() == VerifiedInstructionKind::InitializePlace
                })
                .count(),
            1,
            "Copy array Match result receives real temporary storage"
        );
    }
}

#[test]
fn fresh_indexed_match_base_preserves_owner_through_index_bounds_and_observation() {
    for vector in [false, true] {
        for owned in [false, true] {
            let (text, raw) = structured_owned_fixture::fresh_indexed_match_fixture(vector, owned);
            let sources = sources_for(&text);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated fresh Match base");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{text}\n{errors:?}"));
            let replay = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4, "one Match and one continuation");
            assert_eq!(
                blocks
                    .iter()
                    .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::EnumMatch)
                    .count(),
                1
            );
            let continuation = blocks.last().expect("joined container continuation");
            let instructions = continuation.instructions().collect::<Vec<_>>();
            assert_fresh_access_flow(&instructions, vector, owned);
        }
    }
}

#[test]
fn fresh_indexed_match_base_hostiles_are_exact_deterministic_and_recover() {
    for vector in [false, true] {
        for mismatch in [false, true] {
            let (text, raw) = if mismatch {
                structured_owned_fixture::fresh_indexed_match_mismatch_fixture(vector)
            } else {
                structured_owned_fixture::fresh_indexed_match_implicit_read_fixture(vector)
            };
            let sources = sources_for(&text);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected Match base");
            let reject =
                || lower(pair_input(&syntax, &sources)).expect_err("fresh Match rejection");
            let errors = reject();
            assert_eq!(errors, reject());
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), if mismatch { "ZRYNA-M3017" } else { "ZRYNA-M3013" });
            assert_eq!(
                errors[0].message(),
                if mismatch {
                    "indexed Match base is not yet a prepared exact container"
                } else {
                    "indexed observation requires an explicit clone of its owned element"
                }
            );
            assert_eq!(
                errors[0].guidance(),
                if mismatch {
                    "complete the container Match in an explicit local before indexed access"
                } else {
                    "read a Copy element or explicitly clone the exact owned element"
                }
            );
            let start = text.find("return ").expect("return") + "return ".len();
            let end = text[start..].find(';').expect("terminator") + start;
            let span = errors[0].primary_span().expect("exact indexed expression span");
            assert_eq!((span.start() as usize, span.end() as usize), (start, end));
        }
        let (text, raw) = structured_owned_fixture::fresh_indexed_match_fixture(vector, true);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery fixture");
        lower(pair_input(&syntax, &sources)).expect("valid recovery after hostile Match bases");
    }
}

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
