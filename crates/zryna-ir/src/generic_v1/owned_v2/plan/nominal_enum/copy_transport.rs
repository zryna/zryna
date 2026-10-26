//! Literal nominal CFGs retain independent sealed ownership, identity and loan barriers.
//! These raw witnesses do not claim an original-source or executable-program seal.
use super::super::{graph, raw};
use super::authorities::{Fixture, block, d, ins, int, rejection, stored};
use zryna_layout::{StorageTarget, generic_v1 as layout};

fn edge(target: u32, arguments: &[u32]) -> graph::Edge {
    graph::Edge { target, arguments: arguments.to_vec() }
}

fn witness(f: &Fixture) -> raw::Program {
    let ty = f.marker();
    f.claim(
        vec![
            block(
                0,
                vec![stored(0, 2), stored(1, 0)],
                vec![
                    ins(
                        2,
                        graph::Type::Stored(ty),
                        graph::Operation::ClosedEnumConstruct { ty, ordinal: 0, payload: None },
                    ),
                    ins(3, graph::Type::Stored(ty), graph::Operation::Copy { value: 2 }),
                ],
                graph::Terminator::Branch { condition: 1, yes: edge(1, &[]), no: edge(2, &[]) },
            ),
            block(1, vec![], vec![], graph::Terminator::Jump(edge(3, &[2, 3, 1]))),
            block(2, vec![], vec![], graph::Terminator::Jump(edge(3, &[3, 2, 1]))),
            block(
                3,
                vec![stored(4, ty), stored(5, ty), stored(6, 0)],
                vec![],
                graph::Terminator::Branch { condition: 6, yes: edge(4, &[]), no: edge(5, &[]) },
            ),
            block(
                4,
                vec![],
                vec![ins(7, graph::Type::Stored(0), graph::Operation::BoolLiteral(false))],
                graph::Terminator::Jump(edge(3, &[5, 4, 7])),
            ),
            block(5, vec![], vec![int(8, 7)], graph::Terminator::Return(8)),
        ],
        vec![],
    )
}

#[test]
fn independent_phantom_string_nominal_copy_branch_and_loop_keep_exact_owner_cleanup() {
    let f = Fixture::new();
    let claim = witness(&f);
    f.typed(&claim).expect("sealed Copy nominal identities traverse branch and natural loop");
    let expected = vec![raw::Plan {
        steps: vec![raw::Step {
            block: 5,
            position: 1,
            failure: false,
            end_loans: vec![],
            cleanup: vec![0],
        }],
    }];
    for layouts in [&f.linear, &f.linux] {
        assert_eq!(
            super::super::derive(&claim.graph, &claim.extensions, layouts)
                .expect("exact independent loop cleanup"),
            expected
        );
    }
    let mut other_identity = claim.clone();
    other_identity.graph.functions[0].blocks[3].parameters[0].ty = graph::Type::Stored(f.fixed());
    rejection(f.typed(&other_identity));
    for attack in 0..3 {
        let mut hostile = claim.clone();
        let graph::Terminator::Jump(backedge) =
            &mut hostile.graph.functions[0].blocks[4].terminator
        else {
            panic!("literal backedge");
        };
        match attack {
            0 => {
                backedge.arguments.pop();
            }
            1 => backedge.arguments[0] = u32::MAX,
            _ => backedge.target = 0,
        }
        rejection(f.typed(&hostile));
    }
    let foreign = Fixture::new();
    // Raw DTO bytes carry no source identity. The authority barrier rejects mixing
    // fresh equal-text layout issuers with the retained original SourceMap.
    assert!(
        crate::generic_v1::inventory::check(
            &claim.graph,
            &f.sources,
            &foreign.linear,
            &foreign.linux,
        )
        .is_err(),
        "equal-text foreign sealed layouts cannot be rebound to the original source authority",
    );
    f.typed(&claim).expect("pristine nominal branch/loop after hostile identity retries");
}

#[test]
fn inactive_string_variant_cannot_borrow_copy_authority_from_phantom_shape() {
    let mut f = Fixture::new();
    let copy = witness(&f);
    f.typed(&copy).expect("original phantom String parameter itself is Copy");
    let layout::raw::TypeKind::Enum { variants, .. } = &mut f.graph.types[6].kind else {
        panic!("literal Marker<String>");
    };
    variants[1].payload = Some(layout::raw::NodeId(2));
    f.linear = layout::verify(&f.graph, &f.sources, StorageTarget::Linear32V1)
        .expect("independently valid all-variant String layout");
    f.linux = layout::verify(&f.graph, &f.sources, StorageTarget::LinuxX8664V1)
        .expect("independently valid Linux String layout");
    let mut hostile = copy;
    // Rebind all physical claims so failure depends on independently sealed ownership.
    hostile.graph.type_keys = f.linear.types().map(|ty| ty.key().to_vec()).collect();
    hostile.graph.universe = *f.linear.universe_identity();
    hostile.graph.linear32 = *f.linear.fingerprint();
    hostile.graph.linux_x86_64 = *f.linux.fingerprint();
    assert_ne!(
        f.linear
            .types()
            .find(|ty| ty.id().index() == f.marker())
            .expect("Marker identity")
            .drop_kind(),
        0
    );
    rejection(f.typed(&hostile));
    // Active constructor is still `none`: inactive payload ownership remains decisive.
    let graph::Operation::ClosedEnumConstruct { ordinal, payload, .. } =
        hostile.graph.functions[0].blocks[0].instructions[0].operation
    else {
        panic!("none constructor");
    };
    assert_eq!((ordinal, payload), (0, None));
}

#[test]
fn ordinary_nominal_loan_edges_remain_rejected_by_independent_owner_replay() {
    let f = Fixture::new();
    let loan = graph::Type::Borrow { referent: f.marker(), exclusive: false };
    let claim = f.claim(
        vec![
            block(
                0,
                vec![stored(0, 2), stored(1, 0), d(2, loan)],
                vec![],
                graph::Terminator::Jump(edge(1, &[2, 1])),
            ),
            block(
                1,
                vec![d(3, loan), stored(4, 0)],
                vec![],
                graph::Terminator::Branch { condition: 4, yes: edge(2, &[]), no: edge(3, &[]) },
            ),
            block(
                2,
                vec![],
                vec![ins(5, graph::Type::Stored(0), graph::Operation::BoolLiteral(false))],
                graph::Terminator::Jump(edge(1, &[3, 5])),
            ),
            block(3, vec![], vec![int(6, 7)], graph::Terminator::Return(6)),
        ],
        vec![],
    );
    f.typed(&claim).expect("exact typed nominal loan operands grant no owner replay");
    // Exact typed operands do not grant ordinary phi ownership.
    for layouts in [&f.linear, &f.linux] {
        assert!(
            super::super::derive(&claim.graph, &claim.extensions, layouts).is_err(),
            "nominal identity does not permit ordinary CFG loan transport"
        );
    }
}
