//! Selected-edge state/cleanup owners, using real CFG/metadata kernels but no source proof or seal.
use super::{Failure, plan, raw, typed};
use crate::generic_v1::execution_v1::type_world::TypeWorld;
use typed::borrowed_match_test_support as fixture;
fn run(p: &raw::Program, w: &TypeWorld, work: &mut usize) -> Result<plan::Plan, Failure> {
    let checked = fixture::inputs(p, w)?;
    plan::derive(p, &checked, work)
}
#[test]
fn selected_children_end_before_parents_and_every_closed_head_reaches_exact_exit_state() {
    for family in 0..4 {
        for access in [raw::Access::Shared, raw::Access::Exclusive] {
            let (p, w) = fixture::input(access, family);
            run(&p, &w, &mut 0)
                .expect("selected generated authority only on the active payload edge");
        }
    }
}
#[test]
fn shared_parent_observation_survives_but_exclusive_parent_is_frozen() {
    for access in [raw::Access::Shared, raw::Access::Exclusive] {
        let (mut p, w) = fixture::input(access, 0);
        let f = &mut p.functions[0];
        let enumeration = f.loans[0].referent;
        f.blocks[2].instructions.insert(
            1,
            fixture::instruction(
                raw::Operation::ReadLoan { loan: 0 },
                Some(fixture::value(1, enumeration)),
            ),
        );
        let result = run(&p, &w, &mut 0);
        if access == raw::Access::Shared {
            result.expect("shared selected child coexists with readable parent");
        } else {
            assert!(result.is_err(), "exclusive selected child freezes its exact parent");
        }
    }
}
#[test]
fn ending_parent_before_selected_child_or_omitting_child_end_never_repairs_state() {
    let (p, w) = fixture::input(raw::Access::Shared, 0);
    let mut parent_first = p.clone();
    parent_first.functions[0].blocks[2].instructions.swap(1, 2);
    assert!(
        run(&parent_first, &w, &mut 0).is_err(),
        "shared parent still cannot end before live child"
    );
    let mut missing = p.clone();
    missing.functions[0].blocks[2].instructions.remove(1);
    assert!(
        run(&missing, &w, &mut 0).is_err(),
        "missing selected arm end cannot escape via parent exit"
    );
    let mut leaked = p.clone();
    leaked.functions[0].blocks[2].instructions.truncate(1);
    assert!(
        run(&leaked, &w, &mut 0).is_err(),
        "empty claimed exit cleanup cannot erase live child/parent"
    );
}
#[test]
fn ordinary_edge_other_arm_and_inactive_payload_never_synthesize_selected_authority() {
    let (p, w) = fixture::input(raw::Access::Shared, 0);
    let mut other = p.clone();
    let referent = p.functions[0].loans[1].referent;
    other.functions[0].blocks[1].instructions.insert(
        0,
        fixture::instruction(
            raw::Operation::ReadLoan { loan: 1 },
            Some(fixture::value(1, referent)),
        ),
    );
    assert!(run(&other, &w, &mut 0).is_err(), "payloadless edge has no selected child loan");
    let mut bypass = p.clone();
    let f = &mut bypass.functions[0];
    let boolean = (0..w.len())
        .map(|id| u32::try_from(id).expect("bounded world"))
        .find(|id| w.kind(*id) == Some(crate::generic_v1::execution_v1::type_world::Kind::Bool))
        .expect("actual Bool");
    f.blocks[0].terminator = raw::Terminator::Branch {
        condition: 1,
        yes: raw::Edge { target: 1, arguments: vec![] },
        no: raw::Edge { target: 2, arguments: vec![] },
    };
    f.blocks[0].instructions.push(fixture::instruction(
        raw::Operation::BoolLiteral(true),
        Some(fixture::value(1, boolean)),
    ));
    // A coherent Bool branch reaches both arms but establishes no selected child.
    assert!(run(&bypass, &w, &mut 0).is_err());
    let mut join = p.clone();
    join.functions[0].blocks[2].instructions.truncate(1);
    join.functions[0].blocks[2].terminator =
        raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![] });
    assert!(run(&join, &w, &mut 0).is_err(), "selected and unselected incoming loan states differ");
}
#[test]
fn same_selected_child_can_be_reborrowed_and_must_end_before_the_selected_parent() {
    let (mut p, w) = fixture::input(raw::Access::Exclusive, 0);
    let f = &mut p.functions[0];
    f.loans.push(raw::Loan {
        id: 2,
        referent: f.loans[1].referent,
        access: raw::Access::Shared,
        region: raw::Region { span: fixture::span(24, 30), role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::InternalReborrow(1),
    });
    f.blocks[2].instructions.insert(
        1,
        fixture::instruction(raw::Operation::BeginLoan { loan: 2, failure: None }, None),
    );
    f.blocks[2]
        .instructions
        .insert(2, fixture::instruction(raw::Operation::EndLoan { loan: 2 }, None));
    run(&p, &w, &mut 0).expect("selected exclusive payload supports a shorter shared reborrow");
    let mut wrong = p.clone();
    wrong.functions[0].blocks[2].instructions.swap(2, 3);
    assert!(run(&wrong, &w, &mut 0).is_err(), "nested child must end before selected loan");
}
#[test]
fn selected_state_owner_exact_extra_overflow_and_retry_are_atomic() {
    let (p, w) = fixture::input(raw::Access::Exclusive, 1);
    let checked = fixture::inputs(&p, &w).expect("actual checked selected metadata");
    let mut cost = 0;
    plan::derive(&p, &checked, &mut cost).expect("actual selected state ledger");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    plan::derive(&p, &checked, &mut exact).expect("exact state and cleanup credit");
    assert_eq!(exact, limit);
    for mut work in [limit - cost + 1, usize::MAX] {
        let before = work;
        assert!(plan::derive(&p, &checked, &mut work).is_err());
        assert_eq!(work, before);
    }
    let mut retry = 0;
    plan::derive(&p, &checked, &mut retry).expect("pristine selected state retry");
    assert_eq!(retry, cost);
}

#[test]
fn rooted_selected_payload_has_exact_cached_place_but_no_unborrowed_projection_authority() {
    let (p, w) = fixture::rooted();
    let checked = fixture::inputs(&p, &w).expect("actual rooted enum payload metadata");
    assert_eq!(checked.functions[0].places.loans[1].local_place, Some(1));
    plan::derive(&p, &checked, &mut 0)
        .expect("live selected ancestry authenticates exact payload referent");
    let mut wrong = p.clone();
    wrong.functions[0].blocks[2].instructions[0].operation =
        raw::Operation::ReadPlace { place: 1, mode: raw::ReadMode::Copy };
    assert!(
        run(&wrong, &w, &mut 0).is_err(),
        "scoped Loan refinement does not authorize an unrelated raw Place read"
    );
}
#[test]
fn selected_clone_failure_snapshot_ends_child_before_parent_and_excludes_unpublished_result() {
    let (p, w) = fixture::cloning();
    run(&p, &w, &mut 0).expect("actual fallible clone from selected payload loan");
    for mutate in 0..3 {
        let mut wrong = p.clone();
        let actions = &mut wrong.functions[0].cleanup_plans[1].actions;
        match mutate {
            0 => {
                actions.remove(0);
            }
            1 => actions.swap(0, 1),
            2 => actions.insert(0, raw::CleanupAction::DropPlace(0)),
            _ => unreachable!(),
        }
        assert!(run(&wrong, &w, &mut 0).is_err(), "selected failure snapshot hostile {mutate}");
    }
}

#[test]
fn selected_clone_prefix_snapshot_retains_exact_destination_site_and_loan_order() {
    let (p, w) = fixture::cloning();
    run(&p, &w, &mut 0).expect("fallible clone has both exact outer and prefix cleanup plans");
    for mutate in 0..5 {
        let mut wrong = p.clone();
        let actions = &mut wrong.functions[0].cleanup_plans[2].actions;
        match mutate {
            0 => {
                actions.remove(0);
            }
            1 => actions.swap(1, 2),
            2 => actions[0] = raw::CleanupAction::DropPlace(0),
            3 => {
                actions[0] = raw::CleanupAction::DropInitializedPrefix {
                    destination: 0,
                    preparation_site: raw::PreparationSite {
                        function: 0,
                        block: 2,
                        instruction: 1,
                    },
                }
            }
            4 => {
                actions.remove(1);
            }
            _ => unreachable!(),
        }
        assert!(run(&wrong, &w, &mut 0).is_err(), "selected prefix snapshot hostile {mutate}");
    }
}
