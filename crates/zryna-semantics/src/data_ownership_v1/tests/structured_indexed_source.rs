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
            assert_eq!(instructions[0].kind(), expected);
            let joined = blocks[3].parameters().next().expect("once-evaluated index").id();
            assert_eq!(instructions[0].value_operands().collect::<Vec<_>>(), vec![joined]);
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
            }
            let replay = lower(pair_input(&syntax, &sources)).expect("indexed replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
        }
    }
}
