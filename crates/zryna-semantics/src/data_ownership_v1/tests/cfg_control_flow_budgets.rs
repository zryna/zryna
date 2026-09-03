use super::super::owned_cfg_state::{
    OwnedPendingBlock, release_owned_commit_transition, release_owned_commit_transitions,
    reserve_owned_commit_transition, reserve_owned_commit_transitions,
};
use super::super::owned_string_lowering::loops::preflight_owned_string_loop_skeleton;
use super::*;

#[test]
fn owned_cfg_rejects_duplicate_terminator_and_emission_after_termination() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let terminator = raw::SpannedTerminator {
        span: at,
        kind: raw::Terminator::Return { value: raw::ValueId(0), cleanup: raw::CleanupPlanId(0) },
    };
    assert!(cfg.terminate(terminator.clone(), &mut errors));
    assert!(!cfg.terminate(terminator, &mut errors));
    assert!(!cfg.emit(
        raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        &mut errors,
    ));
    let blocks = cfg.finish(at, &mut errors).expect("one terminated block");
    assert!(blocks[0].instructions.is_empty());
    assert_eq!(blocks[0].terminators.len(), 1);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.code() == "ZRYNA-M3015" && diagnostic.primary_span() == Some(at)
    }));
}

#[test]
fn terminal_owned_if_skeleton_preflight_is_exact_and_atomic() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("terminal if span");

    let mut exact_errors = Errors::new(&sources);
    let exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    let exact_before =
        (exact.arena.blocks.len(), exact.incoming.clone(), exact.edges, exact.transitions);
    assert!(exact.preflight_skeleton(
        zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION,
        at,
        &mut exact_errors,
    ));
    assert_eq!(
        (exact.arena.blocks.len(), exact.incoming.clone(), exact.edges, exact.transitions,),
        exact_before
    );
    assert!(exact_errors.finish().is_empty());

    for (blocks, edges) in [
        (zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION, 0),
        (0, zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION + 1),
        (usize::MAX, usize::MAX),
    ] {
        let mut errors = Errors::new(&sources);
        let state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        let before =
            (state.arena.blocks.len(), state.incoming.clone(), state.edges, state.transitions);
        assert!(!state.preflight_skeleton(blocks, edges, at, &mut errors));
        assert_eq!(
            (state.arena.blocks.len(), state.incoming.clone(), state.edges, state.transitions,),
            before
        );
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_loop_three_block_four_edge_preflight_is_exact_plus_one_and_atomic() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("loop span");
    let maximum_blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION;
    let maximum_edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    exact.arena.blocks.resize_with(maximum_blocks - 3, || OwnedPendingBlock {
        populated: false,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: None,
    });
    exact.incoming.resize(maximum_blocks - 3, 0);
    exact.edges = maximum_edges - 4;
    let before = (exact.arena.blocks.len(), exact.incoming.len(), exact.edges);
    assert!(exact.preflight_skeleton(3, 4, at, &mut exact_errors));
    assert_eq!((exact.arena.blocks.len(), exact.incoming.len(), exact.edges), before);
    assert!(exact_errors.finish().is_empty());

    for (blocks, edges) in [(4, 4), (3, 5), (usize::MAX, usize::MAX)] {
        let mut errors = Errors::new(&sources);
        let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        state.arena.blocks.resize_with(maximum_blocks - 3, || OwnedPendingBlock {
            populated: false,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
        });
        state.incoming.resize(maximum_blocks - 3, 0);
        state.edges = maximum_edges - 4;
        let before = (state.arena.blocks.len(), state.incoming.len(), state.edges);
        assert!(!state.preflight_skeleton(blocks, edges, at, &mut errors));
        assert_eq!((state.arena.blocks.len(), state.incoming.len(), state.edges), before);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }

    let mut errors = Errors::new(&sources);
    let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    state.arena.blocks.resize_with(maximum_blocks - 2, || OwnedPendingBlock {
        populated: false,
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: None,
    });
    state.incoming.resize(maximum_blocks - 2, 0);
    state.edges = maximum_edges - 4;
    let mut known = std::collections::BTreeMap::from([(raw::PlaceId(7), Some(6))]);
    let before = known.clone();
    assert!(!preflight_owned_string_loop_skeleton(&state, &mut known, true, at, &mut errors,));
    assert_eq!(known, before);
}

#[test]
fn owned_loop_commit_transition_reservation_is_exact_plus_one_and_releasable() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("mutation span");
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry block");
    exact.transitions = maximum - 1;
    assert!(reserve_owned_commit_transition(&mut exact, at, &mut exact_errors));
    assert_eq!(exact.transitions, maximum - 1);
    assert_eq!(exact.reserved_transitions, 1);
    release_owned_commit_transition(&mut exact);
    assert_eq!(exact.reserved_transitions, 0);
    assert!(exact_errors.finish().is_empty());

    let mut read_cleanup_errors = Errors::new(&sources);
    let mut read_cleanup =
        OwnedCfgState::single_block(at, &mut read_cleanup_errors).expect("entry block");
    read_cleanup.transitions = maximum - 2;
    assert!(reserve_owned_commit_transitions(&mut read_cleanup, 2, at, &mut read_cleanup_errors,));
    assert_eq!(read_cleanup.reserved_transitions, 2);
    release_owned_commit_transitions(&mut read_cleanup, 2);
    assert_eq!(read_cleanup.reserved_transitions, 0);
    assert!(read_cleanup_errors.finish().is_empty());

    let mut first_extra_errors = Errors::new(&sources);
    let mut first_extra =
        OwnedCfgState::single_block(at, &mut first_extra_errors).expect("entry block");
    first_extra.transitions = maximum - 1;
    let before = (first_extra.transitions, first_extra.reserved_transitions);
    assert!(!reserve_owned_commit_transitions(&mut first_extra, 2, at, &mut first_extra_errors,));
    assert_eq!((first_extra.transitions, first_extra.reserved_transitions), before);
    assert_eq!(first_extra_errors.finish()[0].code(), "ZRYNA-M3201");

    for transitions in [maximum, usize::MAX] {
        let mut errors = Errors::new(&sources);
        let mut state = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
        state.transitions = transitions;
        let before = (state.transitions, state.reserved_transitions);
        assert!(!reserve_owned_commit_transition(&mut state, at, &mut errors));
        assert_eq!((state.transitions, state.reserved_transitions), before);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}

#[test]
fn owned_loop_drop_action_reservation_is_exact_plus_one_overflow_and_releasable() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.span);
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;

    let mut exact_errors = Errors::new(&sources);
    let mut exact = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut exact_errors,
        at,
        maximum - 1,
    );
    assert!(exact.reserve_loop_drop_actions(1, at));
    assert_eq!((exact.cleanup_actions, exact.reserved_cleanup_actions), (maximum - 1, 1));
    exact.release_loop_drop_actions(1);
    assert_eq!((exact.cleanup_actions, exact.reserved_cleanup_actions), (maximum - 1, 0));
    drop(exact);
    assert!(exact_errors.finish().is_empty());

    for current in [maximum, usize::MAX] {
        let mut errors = Errors::new(&sources);
        let mut lowerer = private_string_branch_budget_lowerer(
            input,
            function,
            ty,
            &catalog,
            &mut errors,
            at,
            current,
        );
        let before = (lowerer.cleanup_actions, lowerer.reserved_cleanup_actions);
        assert!(!lowerer.reserve_loop_drop_actions(1, at));
        assert_eq!((lowerer.cleanup_actions, lowerer.reserved_cleanup_actions), before);
        drop(lowerer);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
    }
}
