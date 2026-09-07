use super::*;
use zryna_native_mir::data_ownership_v1::{lower_unverified, raw, verify};

#[test]
fn native_cleanup_claims_are_bound_to_exact_source_ownership_and_budget() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    install(workspace.root(), &registry(), "owned-aggregate");
    let prepared = prepare_data_ownership_for_test(
        &request(workspace.root(), TargetSelection::JavaScript),
        None,
    )
    .unwrap();
    let program = prepared.program();
    let source = program.verified_ir();
    let runtime = program.runtime_abi();
    let claim = lower_unverified(source, runtime).unwrap();
    let fi = claim
        .functions
        .iter()
        .position(|f| {
            f.blocks.iter().any(|b| b.operations.iter().any(|o| o.opcode == raw::Opcode::Construct))
        })
        .unwrap();
    let bi = claim.functions[fi]
        .blocks
        .iter()
        .position(|b| b.operations.iter().any(|o| o.opcode == raw::Opcode::Construct))
        .unwrap();
    let oi = claim.functions[fi].blocks[bi]
        .operations
        .iter()
        .position(|o| o.opcode == raw::Opcode::Construct)
        .unwrap();
    let plan =
        usize::try_from(claim.functions[fi].blocks[bi].operations[oi].cleanup.unwrap()).unwrap();
    assert_eq!(
        claim.functions[fi].cleanup_plans[plan].actions.iter().map(|a| a.place).collect::<Vec<_>>(),
        [1, 0]
    );
    // These two owned literal temporaries remain live until construction succeeds.
    let sealed = source
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedFunction::blocks)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .find(|i| i.kind() == zryna_ir::data_ownership_v1::VerifiedInstructionKind::StructConstruct)
        .unwrap();
    assert_eq!(sealed.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(), [1, 0]);
    for mutation in 0..7 {
        let mut bad = claim.clone();
        let function = &mut bad.functions[fi];
        match mutation {
            0 => function.blocks[bi].operations[oi].cleanup = None,
            1 => function.cleanup_plans[plan].actions.clear(),
            2 => {
                function.cleanup_plans[plan].actions.pop();
            }
            3 => function.cleanup_plans[plan].actions.reverse(),
            4 => {
                let action = function.cleanup_plans[plan].actions[0].clone();
                function.cleanup_plans[plan].actions.push(action);
            }
            5 => {
                function.cleanup_plans[plan].actions[0].place = 3;
            }
            _ => function.blocks[bi].operations[oi].values.reverse(),
        }
        let first = verify(bad.clone(), source, runtime).expect_err("forged source ownership");
        assert_eq!(first, verify(bad, source, runtime).unwrap_err());
        assert!(first.iter().any(|d| d.code() == "ZRYNA-N3116"), "mutation {mutation}: {first:?}");
    }
    let mut exact = claim.clone();
    let function = &mut exact.functions[fi];
    let used = function.cleanup_plans.iter().map(|p| p.actions.len()).sum::<usize>();
    let action = function.cleanup_plans[plan].actions[0].clone();
    function.cleanup_plans.push(raw::CleanupPlan {
        id: u32::try_from(function.cleanup_plans.len()).unwrap(),
        actions: vec![action; zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - used],
    });
    verify(exact.clone(), source, runtime).expect("exact aggregate action budget");
    let extra = exact.functions[fi].cleanup_plans.last_mut().unwrap();
    extra.actions.push(extra.actions[0].clone());
    let failure = verify(exact, source, runtime).expect_err("first extra aggregate cleanup action");
    assert_eq!(failure[0].code(), "ZRYNA-N3201");
    assert_no_artifacts(workspace.root());
}
