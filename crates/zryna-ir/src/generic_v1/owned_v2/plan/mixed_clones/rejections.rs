//! Independent heterogeneous CFG typing and owner/loan replay attacks.

use super::*;

#[test]
fn independent_asymmetric_typing_rejects_swapped_payload_binding_construction_and_leaf() {
    for option_first in [true, false] {
        let (claim, layouts, s) = fixture(option_first);
        let (start, first, direct, direct_first) =
            if option_first { (1, 3, 5, 12) } else { (2, 7, 1, 3) };
        for attack in 0..10 {
            let mut h = claim.clone();
            let blocks = &mut h.graph.functions[0].blocks;
            match attack {
                0 | 1 => {
                    let graph::Terminator::ClosedEnumMatch { arms, .. } = &mut blocks[0].terminator
                    else {
                        panic!("match")
                    };
                    let index = if attack == 0 {
                        usize::from(!option_first)
                    } else {
                        usize::from(option_first)
                    };
                    let target = if attack == 0 { start } else { direct };
                    let ty = borrow(if attack == 0 { 2 } else { 3 });
                    arms[index].binding.as_mut().expect("binding").ty = ty;
                    blocks[target].parameters[0].ty = ty;
                }
                2 | 3 => {
                    let target = if attack == 2 { start + 3 } else { direct };
                    let position = usize::from(attack == 3);
                    let graph::Operation::ClosedEnumConstruct { ordinal, .. } =
                        &mut blocks[target].instructions[position].operation
                    else {
                        panic!("construct")
                    };
                    *ordinal = 1 - *ordinal;
                }
                4 | 5 => {
                    let (result, id) =
                        if attack == 4 { (first + 3, first) } else { (direct_first + 1, first) };
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == result)
                        .expect("leaf")
                        .operation = raw::Operation::CloneBorrowedString(id);
                }
                6 => blocks[start + 2].instructions[0].result.ty = graph::Type::Stored(3),
                7 => {
                    let graph::Terminator::ClosedEnumMatch { arms, .. } = &mut blocks[0].terminator
                    else {
                        panic!("match")
                    };
                    arms[1].ordinal = 0;
                }
                8 => {
                    let graph::Terminator::ClosedEnumMatch { ty, .. } =
                        &mut blocks[start].terminator
                    else {
                        panic!("inner")
                    };
                    *ty = 4;
                }
                _ => {
                    let graph::Operation::ClosedEnumConstruct { payload, .. } =
                        &mut blocks[direct].instructions[1].operation
                    else {
                        panic!("direct")
                    };
                    *payload = Some(0);
                }
            }
            let Failure::Diagnostics(v) =
                typed(&h, &layouts, &s).expect_err("heterogeneous shape attack")
            else {
                panic!("diagnostic")
            };
            assert_eq!(v[0].code, "ZRYNA-I7001");
            typed(&claim, &layouts, &s).expect("pristine typing recovery");
        }
    }
}

#[test]
fn independent_asymmetric_owner_replay_rejects_exclusive_ended_foreign_and_unbalanced_loans() {
    for option_first in [true, false] {
        let (claim, layouts, _) = fixture(option_first);
        let (start, first, direct_first) = if option_first { (1, 3, 12) } else { (2, 7, 3) };
        for attack in 0..7 {
            let mut h = claim.clone();
            match attack {
                0 => {
                    h.extensions[0].retain(|e| e.result != 1 && e.result != 19);
                    h.graph.functions[0].blocks[0].instructions[0] =
                        instruction(1, graph::Type::Stored(1), graph::Operation::I32Literal(0));
                    h.extensions[0].iter_mut().find(|e| e.result == 2).expect("borrow").operation =
                        raw::Operation::Borrow { value: 0, exclusive: true };
                }
                1 => {
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 3)
                        .expect("leaf")
                        .operation = raw::Operation::EndLoan(first + 2);
                    h.graph.functions[0].blocks[start + 2].instructions[0].result.ty =
                        graph::Type::Unit;
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 5)
                        .expect("end")
                        .operation = raw::Operation::CloneBorrowedString(first + 2);
                    h.graph.functions[0].blocks[start + 2].instructions[2].result.ty =
                        graph::Type::Stored(2);
                }
                2 => {
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 5)
                        .expect("end")
                        .operation = raw::Operation::EndLoan(first);
                }
                3 => {
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == direct_first + 1)
                        .expect("leaf")
                        .operation = raw::Operation::CloneBorrowedString(0);
                }
                4 => {
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == direct_first + 3)
                        .expect("end")
                        .operation = raw::Operation::EndLoan(2);
                }
                5 => {
                    h.extensions[0]
                        .iter_mut()
                        .find(|e| e.result == first + 8)
                        .expect("end")
                        .operation = raw::Operation::EndLoan(1);
                }
                _ => {
                    h.extensions[0].iter_mut().find(|e| e.result == 18).expect("drop").operation =
                        raw::Operation::Drop(0);
                }
            }
            let Failure::Diagnostics(v) = derive(&h.graph, &h.extensions, &layouts)
                .expect_err("asymmetric owner/loan attack")
            else {
                panic!("diagnostic")
            };
            assert_eq!(v[0].code, "ZRYNA-I7001");
            derive(&claim.graph, &claim.extensions, &layouts).expect("pristine owner recovery");
        }
    }
}
