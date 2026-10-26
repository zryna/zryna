//! Explicit selected nominal payload owners, failure snapshots and root retention.
use super::super::{Failure, derive, graph, raw};
use super::authorities::*;

pub(super) fn selection(f: &Fixture) -> raw::Program {
    let parameters = vec![stored(0, f.choice()), stored(1, 2)];
    f.claim(
        vec![
            block(
                0,
                parameters,
                vec![],
                graph::Terminator::ClosedEnumMatch {
                    ty: f.choice(),
                    scrutinee: 0,
                    mode: graph::MatchMode::Value,
                    arms: vec![
                        arm(0, None, 1),
                        arm(1, Some(stored(3, 2)), 2),
                        arm(2, Some(stored(6, 1)), 3),
                        arm(3, Some(stored(8, f.nested())), 4),
                    ],
                },
            ),
            block(1, vec![], vec![int(2, 3)], graph::Terminator::Return(2)),
            block(
                2,
                vec![stored(3, 2)],
                vec![ins(4, graph::Type::Stored(2), graph::Operation::Unit), int(5, 7)],
                graph::Terminator::Return(5),
            ),
            block(3, vec![stored(6, 1)], vec![int(7, 23)], graph::Terminator::Return(7)),
            block(
                4,
                vec![stored(8, f.nested())],
                vec![ins(9, graph::Type::Stored(2), graph::Operation::Unit), int(10, 29)],
                graph::Terminator::Return(10),
            ),
        ],
        vec![
            raw::Extension { result: 4, operation: raw::Operation::CloneString(3) },
            raw::Extension {
                result: 9,
                operation: raw::Operation::StringLiteral(b"nested".into()),
            },
        ],
    )
}

#[test]
fn independent_nominal_value_match_transfers_only_active_payload_and_keeps_guard() {
    let f = Fixture::new();
    let claim = selection(&f);
    f.typed(&claim).expect("literal nominal match independently typed on both targets");
    let expected = vec![raw::Plan {
        steps: vec![
            raw::Step {
                block: 1,
                position: 1,
                failure: false,
                end_loans: vec![],
                cleanup: vec![1],
            },
            raw::Step {
                block: 2,
                position: 0,
                failure: true,
                end_loans: vec![],
                cleanup: vec![3, 1],
            },
            raw::Step {
                block: 2,
                position: 2,
                failure: false,
                end_loans: vec![],
                cleanup: vec![4, 3, 1],
            },
            raw::Step {
                block: 3,
                position: 1,
                failure: false,
                end_loans: vec![],
                cleanup: vec![1],
            },
            raw::Step {
                block: 4,
                position: 0,
                failure: true,
                end_loans: vec![],
                cleanup: vec![8, 1],
            },
            raw::Step {
                block: 4,
                position: 2,
                failure: false,
                end_loans: vec![],
                cleanup: vec![9, 8, 1],
            },
        ],
    }];
    for layouts in [&f.linear, &f.linux] {
        assert_eq!(
            derive(&claim.graph, &claim.extensions, layouts).expect("nominal replay"),
            expected
        );
    }
    let mut moved = claim.clone();
    moved.extensions[0][0].operation = raw::Operation::CloneString(0);
    let Failure::Diagnostics(errors) = derive(&moved.graph, &moved.extensions, &f.linear)
        .expect_err("value match consumed the whole enum before exposing a payload")
    else {
        panic!("diagnostic")
    };
    assert_eq!(errors[0].code, "ZRYNA-I7001");
    assert_eq!(
        derive(&claim.graph, &claim.extensions, &f.linear).expect("replay recovery"),
        expected
    );
}

#[test]
fn nominal_owned_cleanup_and_first_extra_state_credit_never_drift_on_retry() {
    let f = Fixture::new();
    let claim = selection(&f);
    let pristine =
        derive(&claim.graph, &claim.extensions, &f.linear).expect("raw independent plan");
    let mut altered = pristine.clone();
    altered[0].steps[1].cleanup.reverse();
    assert_ne!(altered, pristine, "reverse cleanup order is not interchangeable");
    for _ in 0..2 {
        let mut used = super::super::MAX_OWNER_UNITS;
        let Failure::Diagnostics(errors) = super::super::charge(&mut used, 1)
            .expect_err("unchanged complete owner credit ceiling")
        else {
            panic!("diagnostic")
        };
        assert_eq!(errors[0].code, "ZRYNA-I3201");
        assert_eq!(
            derive(&claim.graph, &claim.extensions, &f.linear).expect("pristine retry"),
            pristine
        );
    }
}
