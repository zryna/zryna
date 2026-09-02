use super::*;

#[test]
fn private_string_producer_and_identity_calls_transfer_exact_owners() {
    let (source, raw) = private_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private String calls");
    let program = lower(pair_input(&syntax, &sources)).expect("String producer and identity calls");
    let module = program.modules().next().expect("module");
    let functions = module.functions().collect::<Vec<_>>();
    let caller = &functions[0];
    let identity = &functions[1];
    let producer = &functions[2];
    let caller_block = caller.blocks().next().expect("caller block");
    let calls = caller_block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].callee().expect("producer").declaration(), 2);
    assert_eq!(calls[0].call_arguments().count(), 0);
    assert_eq!(calls[1].callee().expect("identity").declaration(), 1);
    assert_eq!(calls[1].call_arguments().count(), 1);
    for call in &calls {
        assert_eq!(
            call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [1],
            "post-transfer CallTrap retains only the pre-existing survivor"
        );
        assert_eq!(
            caller
                .cleanup_plans()
                .find(|plan| plan.id() == call.cleanup().expect("CallTrap cleanup"))
                .expect("cleanup plan")
                .site()
                .role(),
            VerifiedCleanupRole::CallTrap
        );
    }
    assert!(
        caller_block
            .instructions()
            .any(|instruction| { instruction.kind() == VerifiedInstructionKind::StringClone })
    );
    assert!(identity.places().any(|place| place.kind() == VerifiedPlaceKind::Parameter(0)));
    assert_eq!(
        identity
            .blocks()
            .next()
            .expect("identity block")
            .terminator()
            .derived_drop_actions()
            .count(),
        0
    );
    assert_eq!(
        producer
            .blocks()
            .next()
            .expect("producer block")
            .terminator()
            .derived_drop_actions()
            .count(),
        0
    );
}

#[test]
fn private_string_direct_call_accepts_nested_concat_argument() {
    let (source, raw) = private_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested String call");
    let program = lower(pair_input(&syntax, &sources)).expect("nested String call");
    let caller =
        program.verified_ir().modules().next().expect("module").functions().next().expect("caller");
    let kinds = caller
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&VerifiedInstructionKind::StringConcat));
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::DirectCall).count(),
        1
    );
}
