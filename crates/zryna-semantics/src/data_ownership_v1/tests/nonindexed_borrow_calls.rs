use super::super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/nonindexed-borrow-nested-call.zry");
const SNAPSHOT: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/nonindexed-borrow-nested-call.json");
const EXCLUSIVE_SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/nonindexed-borrow-nested-call-exclusive.zry");
const EXCLUSIVE_SNAPSHOT: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/nonindexed-borrow-nested-call-exclusive.json");

fn lower_fixture(source: &str, snapshot: &[u8]) -> crate::data_ownership_v1::VerifiedProgram {
    let sources = sources_for(source);
    let raw = decode_snapshot(snapshot).expect("non-indexed borrow-call snapshot");
    let syntax = verify_snapshot(raw, &sources).expect("source-authenticated borrow-call fixture");
    lower(pair_input(&syntax, &sources)).expect("nested lexical borrow call")
}

#[test]
fn nested_nonindexed_lexical_call_preserves_authority_and_restores_owner() {
    for _ in 0..2 {
        for (source, snapshot, access) in [
            (SOURCE, SNAPSHOT, VerifiedBorrowAccess::Shared),
            (EXCLUSIVE_SOURCE, EXCLUSIVE_SNAPSHOT, VerifiedBorrowAccess::Exclusive),
        ] {
            let program = lower_fixture(source, snapshot);
            let caller =
                program.modules().next().expect("module").functions().nth(1).expect("caller");
            let instructions = caller
                .blocks()
                .next()
                .expect("structured entry")
                .instructions()
                .collect::<Vec<_>>();
            let begin = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
                .expect("lexical begin");
            let call = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
                .expect("borrowed call");
            let end = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("lexical end");
            let restored = instructions
                .iter()
                .rposition(|instruction| {
                    instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                })
                .expect("post-scope owner move");
            assert!(begin < call && call < end && end < restored);
            let borrow = instructions[begin].borrow().expect("borrow identity");
            assert_eq!(instructions[begin].borrow_access(), Some(access));
            assert_eq!(
                instructions[call].call_arguments().collect::<Vec<_>>(),
                [VerifiedCallArgument::Borrow(borrow)]
            );
            assert_eq!(instructions[end].borrow(), Some(borrow));
            assert_eq!(
                instructions[call]
                    .failure_ended_borrows()
                    .map(zryna_ir::data_ownership_v1::BorrowIdentity::index)
                    .collect::<Vec<_>>(),
                [borrow.index()]
            );
            let cleanup = instructions[call].cleanup().expect("call trap cleanup");
            let plan = caller
                .cleanup_plans()
                .find(|plan| plan.id() == cleanup)
                .expect("call cleanup plan");
            assert_eq!(plan.site().role(), VerifiedCleanupRole::CallTrap);
            assert_eq!(plan.actions().count(), 1, "the caller retains its borrowed owner");
        }
    }
}

#[test]
fn nested_nonindexed_call_does_not_clone_authority_or_fabricate_owned_results() {
    let program = lower_fixture(SOURCE, SNAPSHOT);
    let mut functions = program.modules().next().expect("module").functions();
    let borrowed_function = functions.next().expect("callee");
    let calling_function = functions.next().expect("caller");
    assert_eq!(calling_function.borrow_parameters().count(), 0);
    assert_eq!(borrowed_function.borrow_parameters().count(), 1);
    assert_eq!(
        calling_function
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .count(),
        1
    );
    assert_eq!(
        borrowed_function
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .filter(|instruction| {
                instruction.kind() == VerifiedInstructionKind::GenericCloneBorrow
            })
            .count(),
        1,
        "only the callee creates the distinct owned clone result"
    );
}
