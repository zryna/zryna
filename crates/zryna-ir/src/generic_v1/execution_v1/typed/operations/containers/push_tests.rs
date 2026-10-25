//! Exact original identities and access; no source proof or executable seal is made.
use super::{check, raw};
use crate::generic_v1::execution_v1::typed::{
    Site, container_test_support,
    continuation_test_support::{Types, push},
};
fn run(w: &Types, p: &raw::Program, work: &mut usize) -> Result<bool, super::Failure> {
    let t = container_test_support::phase_inputs(p, &w.world);
    check(
        p,
        &t.functions[0],
        Site { function: 0, block: 0, instruction: 0 },
        &w.world,
        &p.functions[0].blocks[0].instructions[0],
        work,
    )
}
#[test]
fn vec_push_accepts_exact_prepared_rhs_by_place_and_exclusive_formal_loan() {
    let w = Types::new();
    for access in [None, Some(raw::Access::Exclusive)] {
        assert!(run(&w, &push(&w, access), &mut 0).expect("exact Vec<String> push"));
    }
}
#[test]
fn vec_push_rejects_wrong_original_child_head_access_and_owned_result() {
    let w = Types::new();
    let shared_access = push(&w, Some(raw::Access::Shared));
    assert!(run(&w, &shared_access, &mut 0).is_err());
    let mut child = push(&w, None);
    child.functions[0].parameters[1] =
        raw::Parameter::Value(super::super::super::continuation_test_support::value(1, w.integer));
    child.functions[0].places[1].ty = w.integer;
    assert!(run(&w, &child, &mut 0).is_err(), "same storage width cannot replace exact child");
    let mut head = push(&w, None);
    head.functions[0].parameters[0] =
        raw::Parameter::Value(super::super::super::continuation_test_support::value(0, w.shared));
    head.functions[0].places[0].ty = w.shared;
    assert!(run(&w, &head, &mut 0).is_err(), "Shared<String> is not Vec<String>");
    let mut result = push(&w, None);
    result.functions[0].blocks[0].instructions[0].result =
        Some(raw::ValueDefinition { id: 2, ty: raw::ValueType::Unit });
    assert!(run(&w, &result, &mut 0).is_err(), "push publishes no result");
}
#[test]
fn vec_push_type_work_is_exact_extra_overflow_atomic_and_retryable() {
    let w = Types::new();
    let p = push(&w, None);
    let mut cost = 0;
    run(&w, &p, &mut cost).expect("actual type proof");
    assert!(cost > 0);
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&w, &p, &mut exact).expect("exact remaining proof credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(run(&w, &p, &mut spent).is_err());
        assert_eq!(spent, before);
    }
    let mut retry = 0;
    run(&w, &p, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
