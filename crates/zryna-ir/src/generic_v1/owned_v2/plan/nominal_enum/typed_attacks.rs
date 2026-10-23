//! Exact nominal construction, binding and phantom/fixed member affinity.
use super::super::{graph, raw};
use super::authorities::*;

fn constructor(f: &Fixture, ty: u32, ordinal: u32, payload: Option<u32>) -> raw::Program {
    let parameters = payload.map_or_else(Vec::new, |ty| vec![stored(0, ty)]);
    let result = u32::from(payload.is_some());
    let instructions = vec![
        ins(
            result,
            graph::Type::Stored(ty),
            graph::Operation::ClosedEnumConstruct { ty, ordinal, payload: payload.map(|_| 0) },
        ),
        int(result + 1, 7),
    ];
    f.claim(vec![block(0, parameters, instructions, graph::Terminator::Return(result + 1))], vec![])
}

#[test]
fn independent_nominal_tags_and_every_substituted_variant_construct_exactly() {
    let f = Fixture::new();
    for (ty, payloads) in [
        (f.choice(), vec![None, Some(2), Some(1), Some(f.nested())]),
        (f.marker(), vec![None, Some(1)]),
        (f.fixed(), vec![None, Some(2)]),
        (f.plain(), vec![None, Some(2)]),
    ] {
        for (ordinal, payload) in payloads.into_iter().enumerate() {
            let claim = constructor(&f, ty, u32::try_from(ordinal).expect("ordinal"), payload);
            f.typed(&claim).expect("independent literal nominal constructor");
            let at = usize::from(payload.is_some());
            for attack in 0..4 {
                let mut hostile = claim.clone();
                let graph::Operation::ClosedEnumConstruct { ty: claimed, ordinal, payload } =
                    &mut hostile.graph.functions[0].blocks[0].instructions[0].operation
                else {
                    panic!("constructor")
                };
                match attack {
                    0 => *ordinal = u32::MAX,
                    1 => *payload = if payload.is_some() { None } else { Some(0) },
                    2 => *claimed = 1,
                    _ => *claimed = if ty == f.fixed() { f.plain() } else { f.fixed() },
                }
                assert!(
                    f.typed(&hostile).is_err(),
                    "constructor attack {attack} with parameter count {at}"
                );
            }
            f.typed(&claim).expect("pristine nominal constructor replay");
        }
    }
}

#[test]
fn phantom_string_argument_is_copy_but_fixed_string_member_is_affine_even_for_none() {
    let f = Fixture::new();
    for (ty, copy) in [(f.marker(), true), (f.fixed(), false), (f.plain(), false)] {
        let parameters = vec![stored(0, ty)];
        let claim = f.claim(
            vec![block(
                0,
                parameters,
                vec![
                    ins(1, graph::Type::Stored(ty), graph::Operation::Copy { value: 0 }),
                    int(2, 7),
                ],
                graph::Terminator::Return(2),
            )],
            vec![],
        );
        let view = f.linear.types().find(|v| v.id().index() == ty).expect("sealed view");
        assert_eq!(view.drop_kind() == 0, copy, "member affinity, not argument affinity");
        if copy {
            f.typed(&claim).expect("phantom String does not create a String payload");
            let plan = super::super::derive(&claim.graph, &claim.extensions, &f.linear)
                .expect("Copy replay");
            assert!(plan[0].steps[0].cleanup.is_empty());
        } else {
            rejection(f.typed(&claim));
            let none = constructor(&f, ty, 0, None);
            f.typed(&none).expect("none constructor still has its whole affine enum type");
            let plans = super::super::derive(&none.graph, &none.extensions, &f.linear)
                .expect("affine none plan");
            assert_eq!(plans[0].steps[0].cleanup, [0]);
        }
    }
}

#[test]
fn nominal_match_rejects_missing_foreign_swapped_and_inexact_payload_bindings() {
    let f = Fixture::new();
    let claim = super::owner_replay::selection(&f);
    f.typed(&claim).expect("independent complete nominal match");
    for attack in 0..8 {
        let mut hostile = claim.clone();
        let blocks = &mut hostile.graph.functions[0].blocks;
        let graph::Terminator::ClosedEnumMatch { ty, arms, .. } = &mut blocks[0].terminator else {
            panic!("match")
        };
        match attack {
            0 => {
                arms.pop();
            }
            1 => arms[1].ordinal = 0,
            2 => arms[1].binding = None,
            3 => *ty = f.fixed(),
            4 => arms.swap(1, 2),
            5 => {
                arms[1].binding.as_mut().expect("left").ty = graph::Type::Stored(1);
                blocks[2].parameters[0].ty = graph::Type::Stored(1);
            }
            6 => *ty = f.other_choice(),
            _ => {
                arms[3].binding.as_mut().expect("nested").ty = graph::Type::Stored(2);
                blocks[4].parameters[0].ty = graph::Type::Stored(2);
            }
        }
        rejection(f.typed(&hostile));
        f.typed(&claim).expect("pristine complete nominal match after each attack");
    }
}
