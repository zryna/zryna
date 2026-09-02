use super::*;

#[test]
fn generated_program_cfg_budgets_are_checked_at_exact_plus_one_and_overflow() {
    let blocks = zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_PROGRAM;
    let edges = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_PROGRAM;
    assert_eq!(generated_cfg_budget_violation(blocks, edges, 0, 0), None);
    assert_eq!(generated_cfg_budget_violation(blocks - 1, edges, 1, 0), None);
    assert_eq!(
        generated_cfg_budget_violation(blocks, edges, 1, 0),
        Some(ProgramCfgBudgetLimit::Blocks)
    );
    assert_eq!(generated_cfg_budget_violation(blocks, edges - 1, 0, 1), None);
    assert_eq!(
        generated_cfg_budget_violation(blocks, edges, 0, 1),
        Some(ProgramCfgBudgetLimit::Edges)
    );
    assert_eq!(
        generated_cfg_budget_violation(usize::MAX, 0, 1, 0),
        Some(ProgramCfgBudgetLimit::Blocks)
    );
    assert_eq!(
        generated_cfg_budget_violation(0, usize::MAX, 0, 1),
        Some(ProgramCfgBudgetLimit::Edges)
    );
}

#[test]
fn generated_value_composition_counts_only_emitted_definitions_at_exact_boundaries() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("function span");
    let definition =
        |id| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(0), span: at };
    let function = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        entry_export: None,
        span: at,
        parameters: vec![definition(0)],
        borrow_parameters: Vec::new(),
        result: raw::TypeId(0),
        places: Vec::new(),
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: vec![raw::Instruction {
                result: Some(definition(1)),
                span: at,
                kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(0) },
            }],
            terminators: Vec::new(),
        }],
        cleanup_plans: Vec::new(),
    };
    // A fixed-array constant index emits only CopyFromPlace; its literal index is not a value.
    assert_eq!(raw_function_value_count(&function), Some(2));

    let mut vec_index = function.clone();
    vec_index.parameters.clear();
    vec_index.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(definition(0)),
            span: at,
            kind: raw::InstructionKind::I32Literal(0),
        },
        raw::Instruction {
            result: Some(definition(1)),
            span: at,
            kind: raw::InstructionKind::VecIndexCopy {
                place: raw::PlaceId(0),
                index: raw::ValueId(0),
                cleanup: raw::CleanupPlanId(0),
            },
        },
    ];
    // Vec indexing emits both the runtime index value and the checked result.
    assert_eq!(raw_function_value_count(&vec_index), Some(2));

    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_PROGRAM;
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(maximum - 2, &function, &mut exact_errors),
        Some(maximum)
    );
    assert!(exact_errors.finish().is_empty());

    let mut extra_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(maximum - 1, &function, &mut extra_errors),
        None
    );
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut overflow_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_value_function(usize::MAX, &function, &mut overflow_errors),
        None
    );
    assert_eq!(overflow_errors.finish()[0].primary_span(), Some(at));
}

#[test]
fn generated_cfg_edge_table_and_cross_function_first_extra_span_are_exact() {
    let edge = || raw::Edge { target: raw::BlockId(1), arguments: Vec::new() };
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Return {
            value: raw::ValueId(0),
            cleanup: raw::CleanupPlanId(0),
        }),
        0
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Trap {
            identity: raw::TrapIdentity::BoundsV1,
            cleanup: raw::CleanupPlanId(0),
        }),
        0
    );
    assert_eq!(raw_terminator_edge_count(&raw::Terminator::Jump(edge())), 1);
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::Branch {
            condition: raw::ValueId(0),
            when_true: edge(),
            when_false: edge(),
        }),
        2
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::EnumMatch {
            place: raw::PlaceId(0),
            arms: (0..3).map(|variant| raw::EnumArm { variant, edge: edge() }).collect(),
        }),
        3
    );
    assert_eq!(
        raw_terminator_edge_count(&raw::Terminator::WeakUpgradeBranch {
            weak: raw::PlaceId(0),
            success: edge(),
            expired: edge(),
            cleanup: raw::CleanupPlanId(0),
        }),
        2
    );

    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("function span");
    let function = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        entry_export: None,
        span: at,
        parameters: Vec::new(),
        borrow_parameters: Vec::new(),
        result: raw::TypeId(0),
        places: Vec::new(),
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminators: vec![raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(edge()),
            }],
        }],
        cleanup_plans: Vec::new(),
    };
    let maximum = zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_PROGRAM;
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        accumulate_generated_cfg_function(0, maximum - 1, &function, &mut exact_errors),
        Some((1, maximum))
    );
    assert!(exact_errors.finish().is_empty());
    let mut extra_errors = Errors::new(&sources);
    assert_eq!(accumulate_generated_cfg_function(0, maximum, &function, &mut extra_errors), None);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
fn generated_cfg_composition_accepts_multiple_lowered_functions() {
    let (text, raw) = private_string_call_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful multi-function v4");
    let program = lower(pair_input(&syntax, &sources)).expect("multi-function generated CFG");
    let functions = program
        .modules()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
        .collect::<Vec<_>>();
    assert!(functions.len() >= 2);
    assert!(functions.iter().all(|function| function.blocks().len() == 1));
}
