//! Literal accepted constructor-fault identities, independent of generated cleanup claims.
use super::support::{PROVIDERS, seal, source, with_claim};
use zryna_ir::generic_v1::{
    owned_v2::raw::{Extension, Operation, Step},
    raw::Operation as Core,
};

#[test]
fn both_provider_fault_sources_preserve_declaration_order_and_exact_prefix_plans() {
    for case in ["fault-declaration-order", "fault-reordered-spelling"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-struct-v4")
            .join(format!("{case}.zry"));
        assert_eq!(source(case), std::fs::read_to_string(path).expect("frozen fault source"));
        for provider in PROVIDERS {
            with_claim(case, provider, |claim, syntax, sources, linear, linux| {
                assert_eq!(claim.graph.functions.len(), 1);
                assert_eq!(
                    claim.extensions,
                    vec![vec![
                        Extension {
                            result: 0,
                            operation: Operation::StringLiteral(b"earlier-root".to_vec())
                        },
                        Extension {
                            result: 1,
                            operation: Operation::StringLiteral(b"left".to_vec())
                        },
                        Extension { result: 2, operation: Operation::CloneString(0) },
                        Extension {
                            result: 3,
                            operation: Operation::StructConstruct { ty: 3, fields: vec![1, 2] }
                        },
                    ]]
                );
                let body = &claim.graph.functions[0].blocks[0];
                assert_eq!(body.instructions.len(), 5);
                assert_eq!(body.instructions[4].operation, Core::I32Literal(7));
                assert_eq!(
                    claim.plans[0].steps,
                    vec![
                        step(0, true, vec![]),
                        step(1, true, vec![0]),
                        step(2, true, vec![1, 0]),
                        step(5, false, vec![3, 0]),
                    ]
                );
                // Each mutant is physically valid; source/ownership/plan proof must reject.
                for attack in 0..7 {
                    let mut hostile = claim.clone();
                    match attack {
                        0 => {
                            hostile.extensions[0][3].operation =
                                Operation::StructConstruct { ty: 3, fields: vec![2, 1] }
                        }
                        1 => {
                            hostile.extensions[0][3].operation =
                                Operation::StructConstruct { ty: 3, fields: vec![1, 1] }
                        }
                        2 => hostile.plans[0].steps[2].cleanup = vec![3, 1, 0],
                        3 => hostile.plans[0].steps[2].cleanup = vec![2, 1, 0],
                        4 => hostile.plans[0].steps[0].cleanup = vec![1, 0],
                        5 => hostile.plans[0].steps[2].cleanup = vec![0, 1],
                        _ => hostile.plans[0].steps.insert(3, step(3, true, vec![2, 1, 0])),
                    }
                    assert!(
                        seal(&hostile, syntax, sources, linear, linux).is_err(),
                        "{case}/{provider} attack {attack}"
                    );
                }
                seal(&claim, syntax, sources, linear, linux)
                    .expect("pristine claim after all attacks");
            });
        }
    }
}

fn step(position: u32, failure: bool, cleanup: Vec<u32>) -> Step {
    Step { block: 0, position, end_loans: vec![], cleanup, failure }
}
