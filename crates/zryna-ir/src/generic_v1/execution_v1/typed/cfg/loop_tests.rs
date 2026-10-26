//! Actual dominance/reducibility controls; no source or whole execution authority is fabricated.
use super::{Graph, budget, raw};
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
        key: Vec::new(),
        original_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        executable_export: None,
        parameters: Vec::new(),
        result_type: raw::ValueType::Stored(1),
        places: Vec::new(),
        loans: Vec::new(),
        cleanup_plans: Vec::new(),
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: Vec::new(),
            instructions: vec![instruction(0)],
            terminator: raw::Terminator::Return { value: Some(0), cleanup: 0 },
        }],
    }
}
fn graph() -> raw::Function {
    let mut f = function();
    f.blocks[0].instructions.truncate(1);
    f.blocks[0].terminator = raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![0] });
    f.blocks.extend([
        raw::Block {
            id: 1,
            value_parameters: vec![value(1)],
            instructions: Vec::new(),
            terminator: raw::Terminator::Branch {
                condition: 0,
                yes: raw::Edge { target: 2, arguments: Vec::new() },
                no: raw::Edge { target: 3, arguments: Vec::new() },
            },
        },
        raw::Block {
            id: 2,
            value_parameters: Vec::new(),
            instructions: vec![instruction(2)],
            terminator: raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![2] }),
        },
        raw::Block {
            id: 3,
            value_parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: raw::Terminator::Return { value: Some(1), cleanup: 0 },
        },
    ]);
    f
}
#[test]
fn reducible_loop_keeps_header_parameter_and_real_dominance() {
    let f = graph();
    let g = Graph::build(&f, 2, &mut 0).expect("natural loop is admitted independently");
    assert_eq!(g.operand(1, 2, 0).expect("header dominates body"), raw::ValueType::Stored(1));
    assert!(
        g.operand(2, 3, 0).is_err(),
        "body-only definition cannot dominate zero-iteration exit"
    );
}
#[test]
fn independently_forged_irreducible_cycle_and_bad_latch_argument_reject() {
    let mut f = graph();
    f.blocks[0].terminator = raw::Terminator::Branch {
        condition: 0,
        yes: raw::Edge { target: 1, arguments: vec![0] },
        no: raw::Edge { target: 2, arguments: Vec::new() },
    };
    assert!(Graph::build(&f, 2, &mut 0).is_err(), "two-entry cycle is not a structured while");
    let mut f = graph();
    f.blocks[2].terminator = raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![99] });
    assert!(
        Graph::build(&f, 2, &mut 0).is_err(),
        "foreign latch value cannot become a header value"
    );
    Graph::build(&graph(), 2, &mut 0).expect("pristine replay after hostile cyclic claims");
}
#[test]
fn actual_loop_cfg_exact_remaining_extra_overflow_and_retry_are_atomic() {
    let f = graph();
    let mut cost = 0;
    Graph::build(&f, 2, &mut cost).expect("actual loop graph ledger");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    Graph::build(&f, 2, &mut exact).expect("exact remaining loop credit");
    assert_eq!(exact, limit);
    for mut short in [limit - cost + 1, usize::MAX] {
        let before = short;
        assert!(Graph::build(&f, 2, &mut short).is_err());
        assert_eq!(short, before, "failure does not publish debit or a partial graph");
    }
    let mut retry = 0;
    Graph::build(&f, 2, &mut retry).expect("pristine loop graph replay");
    assert_eq!(retry, cost);
}
