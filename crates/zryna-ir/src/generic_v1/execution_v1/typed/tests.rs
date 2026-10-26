//! Private CFG phase controls; these raw rows issue no source, layout or execution authority.
use super::super::raw;
use super::cfg::Graph;

fn value(id: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(1) }
}
fn instruction(id: u32) -> raw::Instruction {
    raw::Instruction {
        site_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        result: Some(value(id)),
        operation: raw::Operation::I32Literal(7),
    }
}
fn function() -> raw::Function {
    raw::Function {
        key: vec![],
        original_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        executable_export: None,
        parameters: vec![],
        result_type: raw::ValueType::Stored(1),
        places: vec![],
        loans: vec![],
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![instruction(0), instruction(1)],
            terminator: raw::Terminator::Return { value: Some(1), cleanup: 0 },
        }],
        cleanup_plans: vec![],
    }
}

#[test]
fn dense_definitions_do_not_grant_forward_or_foreign_value_access() {
    let f = function();
    let graph = Graph::build(&f, 2, &mut 0).expect("isolated complete CFG");
    assert!(graph.operand(1, 0, 0).is_err());
    assert!(graph.operand(99, 0, 2).is_err());
    assert_eq!(graph.operand(0, 0, 1).expect("prior definition"), raw::ValueType::Stored(1));
    let mut duplicate = f;
    duplicate.blocks[0].instructions[1].result = Some(value(0));
    assert!(Graph::build(&duplicate, 2, &mut 0).is_err());
}

#[test]
fn foreign_edges_and_unreachable_claimed_blocks_fail_independently() {
    let mut f = function();
    f.blocks[0].terminator = raw::Terminator::Jump(raw::Edge { target: 99, arguments: vec![] });
    assert!(Graph::build(&f, 2, &mut 0).is_err());
    let mut f = function();
    f.blocks.push(raw::Block {
        id: 1,
        value_parameters: vec![],
        instructions: vec![],
        terminator: raw::Terminator::Return { value: Some(1), cleanup: 0 },
    });
    assert!(Graph::build(&f, 2, &mut 0).is_err());
}

#[test]
fn cfg_retention_budget_overflow_is_atomic_before_pristine_retry() {
    let f = function();
    let mut spent = usize::MAX;
    assert!(Graph::build(&f, 2, &mut spent).is_err());
    assert_eq!(spent, usize::MAX);
    let mut retry = 0;
    Graph::build(&f, 2, &mut retry).expect("fresh CFG retry");
    assert!(retry > 0);
}

#[test]
fn dense_value_permutation_preserves_dominance_and_checks_foreign_use_block() {
    let mut f = function();
    f.blocks[0].instructions.swap(0, 1);
    let graph = Graph::build(&f, 2, &mut 0).expect("dense IDs need not follow visit order");
    assert!(graph.operand(0, 0, 1).is_err());
    assert_eq!(graph.operand(1, 0, 1).expect("definition in first row"), raw::ValueType::Stored(1));
    assert!(graph.operand(1, 99, 2).is_err());
}

#[test]
fn cfg_exact_remaining_limit_and_one_less_are_atomic_with_identical_retry() {
    let f = function();
    let mut cost = 0;
    Graph::build(&f, 2, &mut cost).expect("measure actual bounded graph");
    let limit = super::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    Graph::build(&f, 2, &mut exact).expect("exact remaining limit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(Graph::build(&f, 2, &mut short).is_err());
    assert_eq!(short, before);
    let mut retry = 0;
    Graph::build(&f, 2, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
