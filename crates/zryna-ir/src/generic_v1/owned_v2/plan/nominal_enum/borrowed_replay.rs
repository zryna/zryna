//! Independent nominal shared/exclusive payload refinement and deepest-first loan cleanup.
use super::super::{derive, graph, raw};
use super::authorities::*;

fn selected(f: &Fixture, exclusive: bool) -> raw::Program {
    let loan = |referent| graph::Type::Borrow { referent, exclusive };
    let parameters = vec![stored(0, f.fixed()), stored(1, 2)];
    f.claim(
        vec![
            block(
                0,
                parameters,
                vec![ins(2, loan(f.fixed()), graph::Operation::Unit)],
                graph::Terminator::ClosedEnumMatch {
                    ty: f.fixed(),
                    scrutinee: 2,
                    mode: if exclusive {
                        graph::MatchMode::ExclusiveBorrow
                    } else {
                        graph::MatchMode::SharedBorrow
                    },
                    arms: vec![arm(0, None, 1), arm(1, Some(d(5, loan(2))), 2)],
                },
            ),
            block(
                1,
                vec![],
                vec![ins(3, graph::Type::Unit, graph::Operation::Unit), int(4, 3)],
                graph::Terminator::Return(4),
            ),
            block(
                2,
                vec![d(5, loan(2))],
                vec![
                    ins(6, graph::Type::Stored(2), graph::Operation::Unit),
                    ins(7, graph::Type::Unit, graph::Operation::Unit),
                    ins(8, graph::Type::Unit, graph::Operation::Unit),
                    int(9, 7),
                ],
                graph::Terminator::Return(9),
            ),
        ],
        vec![
            raw::Extension { result: 2, operation: raw::Operation::Borrow { value: 0, exclusive } },
            raw::Extension { result: 3, operation: raw::Operation::EndLoan(2) },
            raw::Extension {
                result: 6,
                operation: if exclusive {
                    raw::Operation::StringLiteral(b"fault".into())
                } else {
                    raw::Operation::CloneBorrowedString(5)
                },
            },
            raw::Extension { result: 7, operation: raw::Operation::EndLoan(5) },
            raw::Extension { result: 8, operation: raw::Operation::EndLoan(2) },
        ],
    )
}

#[test]
fn nominal_borrowed_selection_retains_whole_root_and_ends_deepest_payload_loan_first() {
    let f = Fixture::new();
    for exclusive in [false, true] {
        let claim = selected(&f, exclusive);
        f.typed(&claim).expect("independent nominal payload borrow on both targets");
        let expected = vec![raw::Plan {
            steps: vec![
                raw::Step {
                    block: 1,
                    position: 2,
                    failure: false,
                    end_loans: vec![],
                    cleanup: vec![1, 0],
                },
                raw::Step {
                    block: 2,
                    position: 0,
                    failure: true,
                    end_loans: vec![5, 2],
                    cleanup: vec![1, 0],
                },
                raw::Step {
                    block: 2,
                    position: 4,
                    failure: false,
                    end_loans: vec![],
                    cleanup: vec![6, 1, 0],
                },
            ],
        }];
        for layouts in [&f.linear, &f.linux] {
            assert_eq!(
                derive(&claim.graph, &claim.extensions, layouts)
                    .expect("retained nominal loan plan"),
                expected
            );
        }
        for attack in 0..4 {
            let mut hostile = claim.clone();
            match attack {
                0 => hostile.extensions[0][3].operation = raw::Operation::EndLoan(2),
                1 => hostile.extensions[0][2].operation = raw::Operation::CloneBorrowedString(0),
                2 => hostile.extensions[0][3].operation = raw::Operation::Drop(0),
                _ => hostile.extensions[0][4].operation = raw::Operation::EndLoan(5),
            }
            assert!(
                derive(&hostile.graph, &hostile.extensions, &f.linear).is_err(),
                "nominal loan attack {attack}"
            );
            assert_eq!(
                derive(&claim.graph, &claim.extensions, &f.linear)
                    .expect("pristine nominal loan recovery"),
                expected
            );
        }
    }
}
