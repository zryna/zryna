//! Dense Copy argument fanout isolates this new CFG phase from earlier source payments.
use super::{Graph, budget, raw};
fn function() -> raw::Function {
    let span = zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 };
    raw::Function {
        key: vec![],
        original_span: span,
        executable_export: None,
        parameters: vec![],
        result_type: raw::ValueType::Stored(1),
        places: vec![],
        loans: vec![],
        cleanup_plans: vec![],
        blocks: vec![
            raw::Block {
                id: 0,
                value_parameters: vec![],
                instructions: vec![raw::Instruction {
                    site_span: span,
                    result: Some(raw::ValueDefinition { id: 0, ty: raw::ValueType::Stored(1) }),
                    operation: raw::Operation::I32Literal(7),
                }],
                terminator: raw::Terminator::ClosedEnumMatch {
                    ty: 1,
                    scrutinee: raw::PlaceOrLoan::Place(0),
                    mode: raw::MatchMode::Value,
                    arms: (0..1024)
                        .map(|ordinal| raw::EnumArm {
                            ordinal,
                            binding: None,
                            edge: raw::Edge { target: 1, arguments: vec![0; 256] },
                        })
                        .collect(),
                },
            },
            raw::Block {
                id: 1,
                value_parameters: (1..=256)
                    .map(|id| raw::ValueDefinition { id, ty: raw::ValueType::Stored(1) })
                    .collect(),
                instructions: vec![],
                terminator: raw::Terminator::Return { value: Some(0), cleanup: 0 },
            },
        ],
    }
}
#[test]
fn cfg_argument_fanout_prepays_every_actual_argument_visit() {
    let f = function();
    let mut cost = 0;
    Graph::build(&f, 2, &mut cost).expect("private dense dominance/type-edge kernel");
    assert!(
        cost >= 1024 * 256,
        "earlier source/census work cannot fund this phase's 262144 argument visits"
    );
}
#[test]
fn actual_cfg_fanout_exact_remaining_limit_first_extra_and_overflow_are_atomic() {
    let f = function();
    let mut cost = 0;
    Graph::build(&f, 2, &mut cost).expect("measure actual fanout graph");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    Graph::build(&f, 2, &mut exact).expect("exact remaining fanout credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(Graph::build(&f, 2, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(Graph::build(&f, 2, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    Graph::build(&f, 2, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
