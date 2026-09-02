use super::borrow_parameter_calls::{mixed_snapshot, mixed_source};
use super::*;

const SOURCE: &str = include_str!("../../../../../tests/m3-fixtures/borrow-forwarding-shared.zry");
const JSON: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/borrow-forwarding-shared.json");

#[test]
fn lexical_authority_is_forwarded_unchanged_and_ended_only_by_its_caller() {
    let sources = sources_for(SOURCE);
    let raw = decode_snapshot(JSON).expect("borrow forwarding fixture");
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful borrow forwarding fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("borrow forwarding lowering");
    let module = program.modules().next().expect("module");
    let functions = module.functions().collect::<Vec<_>>();
    let [sink, relay, caller] = functions.as_slice() else { panic!("three functions") };

    let sink_parameter = sink.borrow_parameters().next().expect("sink borrow parameter");
    let relay_parameter = relay.borrow_parameters().next().expect("relay borrow parameter");
    assert_eq!(sink_parameter.id().index(), 0);
    assert_eq!(relay_parameter.id().index(), 0);
    assert_eq!(sink_parameter.access(), VerifiedBorrowAccess::Shared);
    assert_eq!(relay_parameter.access(), VerifiedBorrowAccess::Shared);

    let relay_instructions =
        relay.blocks().next().expect("relay block").instructions().collect::<Vec<_>>();
    let relay_call = relay_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("relay DirectCall");
    assert!(matches!(
        relay_call.call_arguments().nth(2),
        Some(VerifiedCallArgument::Borrow(borrow)) if borrow.index() == relay_parameter.id().index()
    ));
    assert!(!relay_instructions.iter().any(|instruction| {
        matches!(
            instruction.kind(),
            VerifiedInstructionKind::BeginBorrow | VerifiedInstructionKind::EndBorrow
        )
    }));

    let caller_instructions =
        caller.blocks().next().expect("caller block").instructions().collect::<Vec<_>>();
    let begin = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("caller BeginBorrow");
    let call = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("caller DirectCall");
    let end = caller_instructions
        .iter()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("caller EndBorrow");
    let authority = begin.borrow().expect("begun authority");
    assert_eq!(authority.index(), 0);
    assert!(matches!(
        call.call_arguments().nth(2),
        Some(VerifiedCallArgument::Borrow(borrow)) if borrow == authority
    ));
    assert_eq!(end.borrow(), Some(authority));
    let begin_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("begin index");
    let call_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("call index");
    let end_index = caller_instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("end index");
    assert!(begin_index < call_index && call_index < end_index);
}

#[test]
fn forwarding_replay_is_deterministic_after_a_later_borrow_slot_rejection() {
    let arguments = ["left", "left", "right", "exclusive"];
    let rejected_source = mixed_source("exclusive", &arguments, false);
    let rejected_sources = sources_for(&rejected_source);
    let rejected_syntax = verify_snapshot(
        mixed_snapshot(&rejected_source, "exclusive", &arguments, false),
        &rejected_sources,
    )
    .expect("source-faithful rejected forwarding call");
    let diagnostics = lower(pair_input(&rejected_syntax, &rejected_sources))
        .expect_err("later borrow slot rejection");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].message(),
        "borrow arguments must forward an in-scope borrow parameter"
    );

    let accepted = ["left", "shared", "right", "exclusive"];
    let source = mixed_source("exclusive", &accepted, false);
    let sources = sources_for(&source);
    for _ in 0..2 {
        let syntax =
            verify_snapshot(mixed_snapshot(&source, "exclusive", &accepted, false), &sources)
                .expect("source-faithful accepted forwarding call");
        let program = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
        let caller = program.modules().next().expect("module").functions().nth(1).expect("caller");
        assert_eq!(
            caller
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::CopyFromPlace,
                VerifiedInstructionKind::CopyFromPlace,
                VerifiedInstructionKind::DirectCall,
            ]
        );
    }
}
