//! Real CFG/place replay: complete loop owners forget tags, partial owners retain authority.
use super::{plan, raw, typed};
use crate::generic_v1::execution_v1::type_world::TypeWorld;
mod fixture;

fn run(
    p: &raw::Program,
    world: &TypeWorld,
    work: &mut usize,
) -> Result<plan::Plan, super::Failure> {
    let checked = typed::container_test_support::phase_inputs(p, world);
    plan::derive(p, &checked, work)
}

#[test]
fn complete_loop_owner_may_change_closed_variant_without_changing_ownership() {
    let (p, world) = fixture::whole();
    run(&p, &world, &mut 0).expect("different complete variants retain the same exact owner");
    let mut missing = p.clone();
    missing.functions[0].blocks[2].instructions.pop();
    assert!(run(&missing, &world, &mut 0).is_err(), "pending replacement cannot disappear");
    let mut dropped = p.clone();
    dropped.functions[0].blocks[2]
        .instructions
        .push(fixture::instruction(raw::Operation::DropPlace { place: 2 }));
    assert!(run(&dropped, &world, &mut 0).is_err(), "loop owner cannot disappear");
    run(&p, &world, &mut 0).expect("pristine retry");
}

#[test]
fn first_entry_tag_does_not_authorize_payload_access_after_a_repeated_header() {
    let (mut p, world) = fixture::whole();
    let f = &mut p.functions[0];
    let string = f.places[0].ty;
    f.places.push(raw::Place {
        id: 4,
        ty: string,
        span: fixture::span(),
        kind: raw::PlaceKind::EnumPayload { base: 2, variant: 1 },
    });
    f.places.push(raw::Place {
        id: 5,
        ty: string,
        span: fixture::span(),
        kind: raw::PlaceKind::Temporary(4),
    });
    let mut take =
        fixture::instruction(raw::Operation::ReadPlace { place: 4, mode: raw::ReadMode::Move });
    take.result = Some(fixture::value(4, string));
    f.blocks[1].instructions.push(take);
    let Err(super::Failure::Diagnostics(rows)) = run(&p, &world, &mut 0) else {
        panic!("header must reject stale first-entry variant permission");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].code(), "ZRYNA-I7001");
    assert_eq!(rows[0].message(), "inactive or unrefined enum payload place access");
}

#[test]
fn partial_enum_keeps_its_active_variant_and_exact_remaining_payload_cleanup() {
    let (p, world) = fixture::partial();
    run(&p, &world, &mut 0).expect("partial enum retains its exact selected variant");
    let mut wrong = p.clone();
    wrong.functions[0].cleanup_plans[0].actions = vec![raw::CleanupAction::DropPlace(2)];
    assert!(run(&wrong, &world, &mut 0).is_err(), "whole drop cannot cover a moved field");
    run(&p, &world, &mut 0).expect("failed cleanup cannot poison partial authority");
}

#[test]
fn loop_variant_normalization_has_exact_budget_and_atomic_retry() {
    let (p, world) = fixture::whole();
    let mut cost = 0;
    run(&p, &world, &mut cost).expect("actual normalization and state replay");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&p, &world, &mut exact).expect("exact remaining work");
    assert_eq!(exact, limit);
    for mut short in [limit - cost + 1, usize::MAX] {
        let before = short;
        assert!(run(&p, &world, &mut short).is_err());
        assert_eq!(short, before, "no partial work publication");
    }
    let mut retry = 0;
    run(&p, &world, &mut retry).expect("pristine retry after failed normalization");
    assert_eq!(retry, cost);
}
