use super::*;

#[test]
fn owner_state_consumption_removes_every_stale_value_claim() {
    let mut owners = OwnerState::default();
    let owner = raw::PlaceId(7);
    let first = raw::ValueId(11);
    let stale_alias = raw::ValueId(12);
    let _ = owners.register(first, owner);
    owners.value_owners.insert(stale_alias, owner);

    assert!(owners.consume_owner(owner).is_some());
    assert!(!owners.contains(owner));
    assert_eq!(owners.owner(first), None);
    assert_eq!(owners.owner(stale_alias), None);
}

#[test]
fn owner_state_rejects_duplicate_alias_and_self_rehome_without_mutation() {
    let mut owners = OwnerState::default();
    let first_owner = raw::PlaceId(3);
    let second_owner = raw::PlaceId(4);
    let first_value = raw::ValueId(8);
    let second_value = raw::ValueId(9);
    assert!(owners.register(first_value, first_owner).is_some());
    assert!(owners.register(first_value, second_owner).is_none());
    assert!(owners.register(second_value, first_owner).is_none());
    assert!(owners.register_parameter(first_owner).is_none());
    assert_eq!(owners.pending(), &[first_owner]);
    assert_eq!(owners.owner(first_value), Some(first_owner));

    assert!(owners.register(second_value, second_owner).is_some());
    assert!(owners.rename(second_value, first_owner).is_none());
    assert!(owners.rehome_move_result(first_value, first_owner).is_none());
    assert_eq!(owners.pending(), &[first_owner, second_owner]);
    assert_eq!(owners.owner(first_value), Some(first_owner));
    assert_eq!(owners.owner(second_value), Some(second_owner));
}

#[test]
fn owned_cfg_budgets_are_checked_at_exact_plus_one_and_overflow() {
    let blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION;
    let edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(owned_cfg_budget_violation(blocks, edges, transitions), None);
    assert_eq!(
        owned_cfg_budget_violation(blocks + 1, edges, transitions),
        Some(OwnedCfgBudgetLimit::Blocks)
    );
    assert_eq!(
        owned_cfg_budget_violation(blocks, edges + 1, transitions),
        Some(OwnedCfgBudgetLimit::Edges)
    );
    assert_eq!(
        owned_cfg_budget_violation(blocks, edges, transitions + 1),
        Some(OwnedCfgBudgetLimit::Transitions)
    );
    assert_eq!(
        owned_cfg_budget_violation(usize::MAX, usize::MAX, usize::MAX),
        Some(OwnedCfgBudgetLimit::Blocks)
    );
    assert_eq!(dense_owned_value_id(u32::MAX as usize), Some(raw::ValueId(u32::MAX)));
    assert_eq!(dense_owned_value_id(u32::MAX as usize + 1), None);
    assert_eq!(dense_owned_value_id(usize::MAX), None);
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    assert!(!owned_value_budget_violation(values, 0));
    assert!(!owned_value_budget_violation(values - 1, 1));
    assert!(owned_value_budget_violation(values, 1));
    assert!(owned_value_budget_violation(usize::MAX, 1));
    assert!(!owned_place_budget_violation(places, 0));
    assert!(!owned_place_budget_violation(places - 1, 1));
    assert!(owned_place_budget_violation(places, 1));
    assert!(owned_place_budget_violation(usize::MAX, 1));
}

#[test]
fn owned_cfg_value_ledger_is_atomic_for_parameters_blocks_and_results() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let definition = |index| raw::ValueDefinition {
        id: raw::ValueId(u32::try_from(index).expect("bounded value")),
        ty: raw::TypeId(0),
        span: at,
    };

    let mut parameter_errors = Errors::new(&sources);
    let mut parameters = OwnedCfgState::single_block(at, &mut parameter_errors).expect("entry");
    parameters.value_types.resize(maximum - 1, raw::TypeId(0));
    parameters
        .seed_function_parameter(&definition(maximum - 1), &mut parameter_errors)
        .expect("exact value budget");
    assert_eq!(parameters.value_types.len(), maximum);
    assert!(
        parameters.seed_function_parameter(&definition(maximum), &mut parameter_errors).is_none()
    );
    assert_eq!(parameters.value_types.len(), maximum);

    let mut block_errors = Errors::new(&sources);
    let mut blocks = OwnedCfgState::single_block(at, &mut block_errors).expect("entry");
    let successor = blocks.reserve_block(at, &mut block_errors).expect("successor");
    assert!(blocks.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut block_errors,
    ));
    blocks.value_types.resize(maximum - 1, raw::TypeId(0));
    assert!(
        blocks
            .begin_block(
                successor,
                vec![definition(maximum - 1), definition(maximum)],
                at,
                &mut block_errors,
            )
            .is_none()
    );
    assert_eq!(blocks.value_types.len(), maximum - 1);
    assert!(!blocks.arena.blocks[1].populated);

    let mut result_errors = Errors::new(&sources);
    let mut results = OwnedCfgState::single_block(at, &mut result_errors).expect("entry");
    results.value_types.resize(maximum, raw::TypeId(0));
    assert!(!results.emit(
        raw::Instruction {
            result: Some(definition(maximum)),
            span: at,
            kind: raw::InstructionKind::BoolLiteral(true),
        },
        &mut result_errors,
    ));
    assert_eq!(results.value_types.len(), maximum);
    assert_eq!(results.transitions, 0);
    assert!(results.current_block().expect("entry").instructions.is_empty());

    for errors in [parameter_errors, block_errors, result_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
        assert!(diagnostics[0].message().contains("owned CFG values"));
    }
}

#[test]
fn owned_cfg_value_reservation_keeps_child_and_parent_ids_dense() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let definition =
        |id| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(0), span: at };
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.reserve_values(1, at, &mut errors).expect("parent reservation");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(definition(0)),
            span: at,
            kind: raw::InstructionKind::I32Literal(1),
        },
        &mut errors,
    ));
    cfg.release_values(1);
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(definition(1)),
            span: at,
            kind: raw::InstructionKind::I32Literal(2),
        },
        &mut errors,
    ));
    assert_eq!(cfg.value_types.len(), 2);
    assert!(errors.finish().is_empty());

    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let mut limit_errors = Errors::new(&sources);
    let mut limited = OwnedCfgState::single_block(at, &mut limit_errors).expect("entry");
    limited.value_types.resize(maximum - 1, raw::TypeId(0));
    limited.reserve_values(1, at, &mut limit_errors).expect("call-result reservation");
    assert!(!limited.emit(
        raw::Instruction {
            result: Some(definition(u32::try_from(maximum - 1).expect("value id"))),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut limit_errors,
    ));
    assert_eq!(limited.value_types.len(), maximum - 1);
    assert_eq!(limited.transitions, 0);
    assert!(limited.current_block().expect("entry").instructions.is_empty());
    let diagnostics = limit_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn owned_cfg_reserved_local_commit_transition_blocks_initializer_without_mutation() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.transitions = maximum - 1;
    cfg.reserve_transitions(1, at, &mut errors).expect("InitializePlace reservation");
    assert!(!cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(0),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::StringFromUtf8 {
                bytes: b"x".to_vec(),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    assert_eq!(cfg.transitions, maximum - 1);
    assert!(cfg.value_types.is_empty());
    assert!(cfg.current_block().expect("entry").instructions.is_empty());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}
