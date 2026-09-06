use super::*;

#[test]
fn structured_indexed_match_evaluates_once_before_the_only_bounds_site() {
    for vector in [false, true] {
        for owned in [false, true] {
            let (text, raw) = structured_owned_fixture::indexed_match_fixture(vector, owned);
            let sources = sources_for(&text);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated Match index");
            let program = lower(pair_input(&syntax, &sources))
                .expect("Match completes before indexed authority");
            let function = program
                .verified_ir()
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("function");
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4);
            for block in &blocks[..3] {
                assert!(
                    block.instructions().all(|instruction| instruction.indexed_borrow().is_none())
                );
            }
            let instructions = blocks[3].instructions().collect::<Vec<_>>();
            let expected = if vector && !owned {
                VerifiedInstructionKind::VecIndexCopy
            } else {
                VerifiedInstructionKind::BeginIndexedAccess
            };
            assert_eq!(
                blocks
                    .iter()
                    .flat_map(|block| block.instructions())
                    .filter(|instruction| instruction.kind() == expected)
                    .count(),
                1,
                "one bounds operation for the once-evaluated index"
            );
            assert_eq!(instructions[0].kind(), expected);
            let joined = blocks[3].parameters().next().expect("once-evaluated index").id();
            assert_eq!(blocks[3].parameters().count(), 1, "one exact Match handoff");
            assert_eq!(instructions[0].value_operands().collect::<Vec<_>>(), vec![joined]);
            assert_eq!(
                blocks
                    .iter()
                    .flat_map(|block| block.instructions())
                    .filter(|instruction| instruction.result() == Some(joined))
                    .count(),
                0,
                "the joined parameter is reused, not recomputed"
            );
            if owned || !vector {
                assert_eq!(
                    instructions[1].kind(),
                    if owned {
                        VerifiedInstructionKind::GenericCloneBorrow
                    } else {
                        VerifiedInstructionKind::BorrowRead
                    }
                );
                assert_eq!(instructions[2].kind(), VerifiedInstructionKind::EndBorrow);
                assert_eq!(instructions[2].borrow(), instructions[0].borrow());
                if owned {
                    let root = instructions[0].place_operands().next().expect("indexed container");
                    let cleanup = instructions[1].derived_drop_actions().collect::<Vec<_>>();
                    assert!(cleanup.iter().any(|action| action.root() == root));
                    assert!(cleanup.iter().all(|action| action.moved_projections().len() == 0));
                }
            }
            let replay = lower(pair_input(&syntax, &sources)).expect("indexed replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
        }
    }
}

#[test]
fn structured_indexed_static_prefix_precedes_match_and_single_dynamic_bounds() {
    for vector in [false, true] {
        let (text, raw) = structured_owned_fixture::indexed_nested_fixture(vector);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated nested Match index");
        let program =
            lower(pair_input(&syntax, &sources)).expect("static prefix then Match bounds");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let blocks = function.blocks().collect::<Vec<_>>();
        let continuation = blocks.last().expect("continuation");
        let instructions = continuation.instructions().collect::<Vec<_>>();
        assert_eq!(
            blocks
                .iter()
                .flat_map(|block| block.instructions())
                .filter(
                    |instruction| instruction.kind() == VerifiedInstructionKind::BeginIndexedAccess
                )
                .count(),
            1,
            "the static prefix adds no runtime bounds operation"
        );
        let joined = continuation.parameters().next().expect("joined Match index").id();
        assert_eq!(instructions[0].value_operands().collect::<Vec<_>>(), [joined]);
        assert_eq!(instructions[0].kind(), VerifiedInstructionKind::BeginIndexedAccess);
        assert_eq!(instructions[1].kind(), VerifiedInstructionKind::BorrowRead);
        assert_eq!(instructions[2].kind(), VerifiedInstructionKind::EndBorrow);
    }
}
