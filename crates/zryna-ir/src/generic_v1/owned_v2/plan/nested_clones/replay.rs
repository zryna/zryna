//! Independent nested CFG typing, retained roots and live-loan replay.

use super::*;

#[test]
fn independent_nested_selected_paths_retain_root_and_end_deepest_loans_before_cleanup() {
    for result in [false, true] {
        let (claim, layouts, s) = fixture(result);
        typed(&claim, &layouts, &s).expect("fixed independent typed nested CFG");
        let plans = derive(&claim.graph, &claim.extensions, &layouts)
            .expect("independent owner/loan replay");
        let failures = plans[0].steps.iter().filter(|s| s.failure).collect::<Vec<_>>();
        let expected = if result {
            vec![(3, vec![5, 3, 2, 1]), (7, vec![14, 12, 2, 1])]
        } else {
            vec![(4, vec![6, 4, 2, 1])]
        };
        assert_eq!(failures.len(), expected.len());
        for (step, (block, loans)) in failures.iter().zip(expected) {
            assert_eq!(step.block, block);
            assert_eq!(step.position, 0);
            assert_eq!(step.end_loans, loans);
            assert_eq!(step.cleanup, [0]);
        }
        let last = plans[0].steps.last().expect("return");
        assert!(!last.failure);
        assert!(last.end_loans.is_empty());
        assert_eq!(last.cleanup, [0]);
    }
}

#[test]
fn independent_nested_typed_leaf_rejects_outer_inner_and_fresh_owner_operands() {
    for result in [false, true] {
        let (claim, layouts, s) = fixture(result);
        let first = if result { 3 } else { 4 };
        for id in [0, 2, first, first + 3] {
            let mut hostile = claim.clone();
            hostile.extensions[0]
                .iter_mut()
                .find(|e| e.result == first + 3)
                .expect("clone")
                .operation = raw::Operation::CloneBorrowedString(id);
            assert!(typed(&hostile, &layouts, &s).is_err());
            typed(&claim, &layouts, &s).expect("pristine retry");
        }
    }
}

#[test]
fn independent_nested_typing_rejects_forged_family_ordinal_payload_and_binding() {
    for result in [false, true] {
        let (claim, layouts, sources) = fixture(result);
        let (start, first) = if result { (1, 3) } else { (2, 4) };
        for attack in 0..10 {
            let mut hostile = claim.clone();
            let blocks = &mut hostile.graph.functions[0].blocks;
            match attack {
                0 | 1 | 7 | 8 => {
                    let graph::Terminator::ClosedEnumMatch { ty, scrutinee, arms, .. } =
                        &mut blocks[start].terminator
                    else {
                        panic!("inner match")
                    };
                    match attack {
                        0 => *ty = 4,
                        1 => *scrutinee = 2,
                        7 => arms[1].binding = None,
                        _ => arms[1].ordinal = 0,
                    }
                }
                2 => {
                    blocks[start].parameters[0].ty = borrow(2);
                    let graph::Terminator::ClosedEnumMatch { arms, .. } = &mut blocks[0].terminator
                    else {
                        panic!("outer match")
                    };
                    arms[usize::from(!result)].binding.as_mut().expect("payload").ty = borrow(2);
                }
                3 => {
                    blocks[start + 1].instructions[0].operation =
                        graph::Operation::ClosedEnumConstruct {
                            ty: 3,
                            ordinal: 0,
                            payload: Some(0),
                        }
                }
                4 => {
                    blocks[start + 2].instructions[1].operation =
                        graph::Operation::ClosedEnumConstruct {
                            ty: 3,
                            ordinal: 0,
                            payload: Some(first + 3),
                        }
                }
                5 => {
                    blocks[start + 2].instructions[1].operation =
                        graph::Operation::ClosedEnumConstruct {
                            ty: 4,
                            ordinal: 1,
                            payload: Some(first + 3),
                        }
                }
                6 => {
                    blocks[start + 3].instructions[0].operation =
                        graph::Operation::ClosedEnumConstruct {
                            ty: 4,
                            ordinal: u32::from(!result),
                            payload: Some(0),
                        }
                }
                _ => blocks[start + 2].instructions[0].result.ty = graph::Type::Stored(3),
            }
            let Failure::Diagnostics(errors) = typed(&hostile, &layouts, &sources)
                .expect_err("independent nested identity/shape attack")
            else {
                panic!("typed diagnostic")
            };
            assert_eq!(errors[0].code, "ZRYNA-I7001");
            typed(&claim, &layouts, &sources).expect("pristine after every shape mutation");
        }
    }
}

#[test]
fn independent_nested_planner_rejects_exclusive_ended_foreign_loans_and_bad_end_order() {
    for result in [false, true] {
        let (claim, layouts, _) = fixture(result);
        let first = if result { 3 } else { 4 };
        let some = if result { 3 } else { 4 };
        let output = if result { 21 } else { 13 };
        for attack in 0..6 {
            let mut hostile = claim.clone();
            match attack {
                0 => {
                    hostile.extensions[0].retain(|e| e.result != 1 && e.result != output + 3);
                    hostile.graph.functions[0].blocks[0].instructions[0] =
                        instruction(1, graph::Type::Stored(1), graph::Operation::I32Literal(0));
                    hostile.extensions[0][0].operation =
                        raw::Operation::Borrow { value: 0, exclusive: true };
                }
                1 => {
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 3)
                        .expect("clone")
                        .operation = raw::Operation::EndLoan(first + 2);
                    hostile.graph.functions[0].blocks[some].instructions[0].result.ty =
                        graph::Type::Unit;
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 5)
                        .expect("end")
                        .operation = raw::Operation::CloneBorrowedString(first + 2);
                    hostile.graph.functions[0].blocks[some].instructions[2].result.ty =
                        graph::Type::Stored(2);
                }
                2 => {
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 3)
                        .expect("clone")
                        .operation = raw::Operation::CloneBorrowedString(0);
                }
                3 => {
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 5)
                        .expect("end")
                        .operation = raw::Operation::EndLoan(2);
                }
                4 => {
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 5)
                        .expect("end")
                        .operation = raw::Operation::EndLoan(1);
                }
                _ => {
                    hostile.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == output + 2)
                        .expect("drop")
                        .operation = raw::Operation::Drop(0);
                }
            }
            let Failure::Diagnostics(errors) =
                derive(&hostile.graph, &hostile.extensions, &layouts)
                    .expect_err("nested planner attack")
            else {
                panic!("diagnostic")
            };
            assert_eq!(errors[0].code, "ZRYNA-I7001");
            if attack == 0 {
                assert_eq!(errors[0].message, "selected clone requires a live shared String loan");
            }
            derive(&claim.graph, &claim.extensions, &layouts)
                .expect("pristine retry after every hostile replay");
        }
    }
}
