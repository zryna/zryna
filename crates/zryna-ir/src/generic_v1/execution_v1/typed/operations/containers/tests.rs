//! Independent concrete type/frontier kernels; no source proof or executable seal is made.
use super::{check, raw};
use crate::generic_v1::execution_v1::frontier;
use crate::generic_v1::execution_v1::{
    type_world::{Builder, Frame, Kind, TypeWorld},
    typed::{Site, container_test_support},
};
mod fixture;
use fixture::{Fixture, instruction, program};
fn run(
    w: &Fixture,
    params: &[u32],
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<bool, super::Failure> {
    let p = program(&w.world, params, i);
    let typed = container_test_support::phase_inputs(&p, &w.world);
    check(
        &p,
        &typed.functions[0],
        Site { function: 0, block: 0, instruction: 0 },
        &w.world,
        i,
        work,
    )
}
#[test]
fn container_typing_rejects_wrong_kind_element_payload_result_and_exact_referent() {
    let w = Fixture::new();
    let vector = instruction(
        2,
        w.vector,
        raw::Operation::VecConstruct { ty: w.vector, elements: vec![0, 1], failure: 0 },
    );
    assert!(run(&w, &[w.string, w.string], &vector, &mut 0).expect("exact Vec<String>"));
    let empty = instruction(
        0,
        w.vector,
        raw::Operation::VecConstruct { ty: w.vector, elements: vec![], failure: 0 },
    );
    assert!(run(&w, &[], &empty, &mut 0).expect("empty Vec still has exact preparation snapshot"));
    let wrong_element = instruction(
        2,
        w.vector_integer,
        raw::Operation::VecConstruct { ty: w.vector_integer, elements: vec![0, 1], failure: 0 },
    );
    assert!(run(&w, &[w.string, w.string], &wrong_element, &mut 0).is_err());
    let wrong_kind = instruction(
        1,
        w.shared,
        raw::Operation::VecConstruct { ty: w.shared, elements: vec![0], failure: 0 },
    );
    assert!(run(&w, &[w.string], &wrong_kind, &mut 0).is_err());
    let shared = instruction(
        1,
        w.shared,
        raw::Operation::SharedConstruct { ty: w.shared, value: 0, failure: 0 },
    );
    assert!(run(&w, &[w.string], &shared, &mut 0).expect("exact Shared payload move"));
    assert!(run(&w, &[w.integer], &shared, &mut 0).is_err());
    let weak = instruction(
        1,
        w.weak,
        raw::Operation::WeakDowngrade { shared: raw::PlaceOrLoan::Place(0), failure: 0 },
    );
    assert!(run(&w, &[w.shared], &weak, &mut 0).expect("same exact referent"));
    assert!(run(&w, &[w.weak], &weak, &mut 0).is_err(), "downgrade requires Shared source head");
    let wrong = instruction(1, w.weak_other, weak.operation.clone());
    assert!(
        run(&w, &[w.shared_nominal], &wrong, &mut 0).is_err(),
        "equal-shaped nominal referents remain distinct"
    );
    let mut absent = shared.clone();
    absent.result = None;
    assert!(run(&w, &[w.string], &absent, &mut 0).is_err());
    let mut result = shared.clone();
    result.result.as_mut().expect("result").ty = raw::ValueType::Stored(w.vector);
    assert!(run(&w, &[w.string], &result, &mut 0).is_err());
}
#[test]
fn actual_vec_operand_scan_has_exact_remaining_credit_atomic_extra_overflow_retry() {
    let w = Fixture::new();
    let params = vec![w.integer; 16];
    let i = instruction(
        16,
        w.vector_integer,
        raw::Operation::VecConstruct {
            ty: w.vector_integer,
            elements: (0..16).collect(),
            failure: 0,
        },
    );
    let mut cost = 0;
    run(&w, &params, &i, &mut cost).expect("actual operand fanout");
    assert!(cost >= 16);
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&w, &params, &i, &mut exact).expect("exact remaining credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(run(&w, &params, &i, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(run(&w, &params, &i, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    run(&w, &params, &i, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
#[test]
fn count_frontiers_never_require_or_traverse_opaque_payload_clone_and_vec_cycles_are_finite() {
    let mut b = Builder::new();
    let opaque = b
        .declare(
            Kind::Opaque { owner: Frame { kind: 0, module: 0, index: 0 }, slot: 0 },
            &[],
            &mut 0,
        )
        .expect("original opaque T");
    let shared = b.declare(Kind::Shared, &[opaque], &mut 0).expect("Shared<T>");
    let weak = b.declare(Kind::Weak, &[opaque], &mut 0).expect("Weak<T>");
    let forbidden = b.declare(Kind::Vec, &[opaque], &mut 0).expect("Vec<T>");
    let node =
        b.declare(Kind::Struct { module: 0, declaration: 0, arity: 0 }, &[], &mut 0).expect("Node");
    let vector = b.declare(Kind::Vec, &[node], &mut 0).expect("Vec<Node>");
    b.set_members(node, &[Some(vector)], &mut 0).expect("real indirect member");
    let world = b.seal(&mut 0).expect("separate argument/storage/capability checks");
    assert!(!world.fact(opaque).expect("opaque fact").clone);
    let table = frontier::Table::build(&world, &mut 0).expect("finite checked graph");
    for (ty, kind) in
        [(shared, frontier::CloneNodeKind::SharedCount), (weak, frontier::CloneNodeKind::WeakCount)]
    {
        let n = table.node(ty).expect("unconditional count frontier");
        assert_eq!(n.kind, kind);
        assert!(n.children.is_empty());
        assert!(n.may_fail);
    }
    assert!(table.node(forbidden).is_err(), "Vec requires original element Clone");
    for ty in [shared, forbidden] {
        let operation = if ty == shared {
            raw::Operation::SharedConstruct { ty, value: 0, failure: 0 }
        } else {
            raw::Operation::VecConstruct { ty, elements: vec![0], failure: 0 }
        };
        let i = instruction(1, ty, operation);
        let p = program(&world, &[opaque], &i);
        let typed = container_test_support::phase_inputs(&p, &world);
        check(
            &p,
            &typed.functions[0],
            Site { function: 0, block: 0, instruction: 0 },
            &world,
            &i,
            &mut 0,
        )
        .expect("original opaque payload moves impose no Clone requirement");
    }

    assert_eq!(table.node(node).expect("Node").children, vec![Some(vector)]);
    assert_eq!(table.node(vector).expect("Vec<Node>").children, vec![Some(node)]);
    assert!(table.node(node).expect("Node failure frontier").may_fail);
}
#[test]
fn vec_index_accepts_only_original_copy_element_and_exact_i32_index() {
    let w = Fixture::new();
    for (vector, element, accepted) in
        [(w.vector_integer, w.integer, true), (w.vector, w.string, false)]
    {
        let i = instruction(
            2,
            element,
            raw::Operation::IndexCopy { base: raw::PlaceOrLoan::Place(0), index: 1, failure: 0 },
        );
        let p = program(&w.world, &[vector, w.integer], &i);
        let typed = container_test_support::phase_inputs(&p, &w.world);
        assert_eq!(
            super::super::actions::index(
                &p,
                &typed.functions[0],
                Site { function: 0, block: 0, instruction: 0 },
                &w.world,
                &i,
                &mut 0
            )
            .is_ok(),
            accepted
        );
        let wrong = program(&w.world, &[vector, w.string], &i);
        let typed = container_test_support::phase_inputs(&wrong, &w.world);
        assert!(
            super::super::actions::index(
                &wrong,
                &typed.functions[0],
                Site { function: 0, block: 0, instruction: 0 },
                &w.world,
                &i,
                &mut 0
            )
            .is_err()
        );
    }
}
