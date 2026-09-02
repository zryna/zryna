use super::*;

#[test]
fn owned_cfg_enforces_each_storage_limit_at_the_emission_site() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut block_errors = Errors::new(&sources);
    let mut blocks = OwnedCfgState::single_block(at, &mut block_errors).expect("entry block");
    for _ in 1..zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION {
        blocks.reserve_block(at, &mut block_errors).expect("exact block budget");
    }
    assert!(blocks.reserve_block(at, &mut block_errors).is_none());

    let mut edge_errors = Errors::new(&sources);
    let mut edges = OwnedCfgState::single_block(at, &mut edge_errors).expect("entry block");
    let successor = edges.reserve_block(at, &mut edge_errors).expect("reserved successor");
    edges.edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;
    assert!(!edges.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut edge_errors,
    ));

    let mut transition_errors = Errors::new(&sources);
    let mut transitions =
        OwnedCfgState::single_block(at, &mut transition_errors).expect("entry block");
    transitions.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!transitions.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut transition_errors,
    ));

    for errors in [block_errors, edge_errors, transition_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_cfg_reserves_then_populates_a_canonical_multiblock_skeleton() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let left = cfg.reserve_block(at, &mut errors).expect("left reservation");
    let join = cfg.reserve_block(at, &mut errors).expect("join reservation");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Branch {
                condition: raw::ValueId(0),
                when_true: raw::Edge { target: left, arguments: Vec::new() },
                when_false: raw::Edge { target: join, arguments: Vec::new() },
            },
        },
        &mut errors,
    ));
    cfg.begin_block(left, Vec::new(), at, &mut errors).expect("left block");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: join, arguments: Vec::new() }),
        },
        &mut errors,
    ));
    cfg.begin_block(join, Vec::new(), at, &mut errors).expect("join block");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("complete skeleton");
    assert_eq!(blocks.iter().map(|block| block.id.0).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(errors.finish().is_empty());
}

#[test]
fn owned_cfg_finish_rejects_unterminated_and_unpopulated_blocks_with_m3015() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut unterminated_errors = Errors::new(&sources);
    let unterminated =
        OwnedCfgState::single_block(at, &mut unterminated_errors).expect("entry block");
    assert!(unterminated.finish(at, &mut unterminated_errors).is_none());

    let mut unpopulated_errors = Errors::new(&sources);
    let mut unpopulated =
        OwnedCfgState::single_block(at, &mut unpopulated_errors).expect("entry block");
    unpopulated.reserve_block(at, &mut unpopulated_errors).expect("successor reservation");
    assert!(unpopulated.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut unpopulated_errors,
    ));
    assert!(unpopulated.finish(at, &mut unpopulated_errors).is_none());

    for errors in [unterminated_errors, unpopulated_errors] {
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_cfg_rejects_invalid_targets_and_switch_before_termination() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    for target in [raw::BlockId(0), raw::BlockId(u32::MAX)] {
        let mut errors = Errors::new(&sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        assert!(!cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target, arguments: Vec::new() }),
            },
            &mut errors,
        ));
        assert_eq!(errors.finish()[0].code(), "ZRYNA-M3015");
    }

    let mut current_errors = Errors::new(&sources);
    let mut invalid_current =
        OwnedCfgState::single_block(at, &mut current_errors).expect("entry block");
    invalid_current.current = None;
    assert!(!invalid_current.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut current_errors,
    ));
    assert_eq!(current_errors.finish()[0].code(), "ZRYNA-M3015");

    let mut switch_errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut switch_errors).expect("entry block");
    let successor = cfg.reserve_block(at, &mut switch_errors).expect("successor");
    assert!(cfg.begin_block(successor, Vec::new(), at, &mut switch_errors).is_none());
    assert_eq!(switch_errors.finish()[0].code(), "ZRYNA-M3015");
}

#[test]
fn owned_cfg_reservation_preserves_dense_global_value_definition_order() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("parameter-seeded entry");
    cfg.seed_function_parameter(
        &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
        &mut errors,
    )
    .expect("function parameter");
    let successor = cfg.reserve_block(at, &mut errors).expect("identity-only reservation");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(1),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::StringFromUtf8 {
                bytes: vec![b'x'],
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge {
                target: successor,
                arguments: vec![raw::ValueId(1)],
            }),
        },
        &mut errors,
    ));
    cfg.begin_block(
        successor,
        vec![raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span: at }],
        at,
        &mut errors,
    )
    .expect("successor parameter");
    assert!(cfg.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(3),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut errors,
    ));
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(3),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("complete value-ordered CFG");
    assert_eq!(blocks[0].instructions[0].result.as_ref().expect("entry value").id.0, 1);
    assert_eq!(blocks[1].parameters[0].id.0, 2);
    assert_eq!(blocks[1].instructions[0].result.as_ref().expect("successor value").id.0, 3);
    assert!(errors.finish().is_empty());
}

#[test]
fn owned_cfg_failed_value_mutations_preserve_state_and_close_parameter_seeding() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let successor = cfg.reserve_block(at, &mut errors).expect("successor");
    assert!(cfg.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: successor, arguments: Vec::new() }),
        },
        &mut errors,
    ));
    let before = (cfg.current, cfg.value_types.clone(), cfg.arena.blocks[1].populated);
    assert!(
        cfg.begin_block(
            successor,
            vec![
                raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
                raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span: at },
            ],
            at,
            &mut errors,
        )
        .is_none()
    );
    assert_eq!((cfg.current, cfg.value_types.clone(), cfg.arena.blocks[1].populated), before);

    let mut transition_errors = Errors::new(&sources);
    let mut transitions =
        OwnedCfgState::single_block(at, &mut transition_errors).expect("entry block");
    transitions.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let before = (
        transitions.value_types.clone(),
        transitions.arena.blocks[0].instructions.len(),
        transitions.function_parameters_open,
    );
    assert!(!transitions.emit(
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(0),
                ty: raw::TypeId(0),
                span: at,
            }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut transition_errors,
    ));
    assert_eq!(
        (
            transitions.value_types.clone(),
            transitions.arena.blocks[0].instructions.len(),
            transitions.function_parameters_open,
        ),
        before
    );

    let mut late_errors = Errors::new(&sources);
    let mut late = OwnedCfgState::single_block(at, &mut late_errors).expect("entry block");
    assert!(late.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut late_errors,
    ));
    assert!(
        late.seed_function_parameter(
            &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
            &mut late_errors,
        )
        .is_none()
    );
    assert_eq!(late_errors.finish()[0].code(), "ZRYNA-M3015");
}

#[test]
fn owned_cfg_finish_rejects_disconnected_cycles_and_edge_signature_mismatch() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");

    let mut cycle_errors = Errors::new(&sources);
    let mut cycle = OwnedCfgState::single_block(at, &mut cycle_errors).expect("entry block");
    let left = cycle.reserve_block(at, &mut cycle_errors).expect("left");
    let right = cycle.reserve_block(at, &mut cycle_errors).expect("right");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut cycle_errors,
    ));
    cycle.begin_block(left, Vec::new(), at, &mut cycle_errors).expect("left block");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: right, arguments: Vec::new() }),
        },
        &mut cycle_errors,
    ));
    cycle.begin_block(right, Vec::new(), at, &mut cycle_errors).expect("right block");
    assert!(cycle.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target: left, arguments: Vec::new() }),
        },
        &mut cycle_errors,
    ));
    assert!(cycle.finish(at, &mut cycle_errors).is_none());
    assert_eq!(cycle_errors.finish()[0].code(), "ZRYNA-M3015");

    let mut signature_errors = Errors::new(&sources);
    let mut signature =
        OwnedCfgState::single_block(at, &mut signature_errors).expect("entry block");
    signature
        .seed_function_parameter(
            &raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(0), span: at },
            &mut signature_errors,
        )
        .expect("function parameter");
    let target = signature.reserve_block(at, &mut signature_errors).expect("target");
    assert!(signature.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Jump(raw::Edge { target, arguments: vec![raw::ValueId(0)] }),
        },
        &mut signature_errors,
    ));
    signature
        .begin_block(
            target,
            vec![raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span: at }],
            at,
            &mut signature_errors,
        )
        .expect("target signature");
    assert!(signature.terminate(
        raw::SpannedTerminator {
            span: at,
            kind: raw::Terminator::Return {
                value: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        &mut signature_errors,
    ));
    assert!(signature.finish(at, &mut signature_errors).is_none());
    assert_eq!(signature_errors.finish()[0].code(), "ZRYNA-M3015");
}
