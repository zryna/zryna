//! Real independent state/plan kernels; fixtures construct no `SourceProof` or execution seal.
use super::{Context, Failure, State, instructions, plan, raw};
mod fixture;
use fixture::{Case, inputs, program, span};
fn message(failure: Failure, needle: &str) {
    let Failure::Diagnostics(diagnostics) = failure else { panic!("owning diagnostic expected") };
    assert!(
        diagnostics.iter().any(|d| d.message.contains(needle)),
        "expected owning gate {needle}, got {diagnostics:?}"
    );
}
#[test]
fn containers_accept_exact_precommit_failure_snapshots_and_publish_result_last() {
    for case in [Case::Vector, Case::Shared, Case::Downgrade] {
        let (p, world) = program(case);
        let typed = inputs(&p, &world);
        let plan = plan::derive(&p, &typed, &mut 0)
            .expect("actual state/plan accepts settled container operation");
        assert_eq!(plan.operations[0].failure, Some(0));
        assert_eq!(
            plan.operations[0].destination,
            Some(u32::try_from(p.functions[0].places.len() - 1).expect("small fixture"))
        );
        assert_eq!(
            plan.operations[0].prefix, None,
            "constructor/count acquisition has no partial child prefix"
        );
    }
}
#[test]
fn failure_cleanup_cannot_omit_reorder_or_include_unwritten_container_destination() {
    let (p, world) = program(Case::Vector);
    for actions in [
        vec![raw::CleanupAction::DropPlace(0)],
        vec![raw::CleanupAction::DropPlace(0), raw::CleanupAction::DropPlace(1)],
        vec![
            raw::CleanupAction::DropPlace(2),
            raw::CleanupAction::DropPlace(1),
            raw::CleanupAction::DropPlace(0),
        ],
    ] {
        let mut bad = p.clone();
        bad.functions[0].cleanup_plans[0].actions = actions;
        message(
            plan::derive(&bad, &inputs(&bad, &world), &mut 0)
                .expect_err("exact precommit owners required"),
            "cleanup differs from independent state replay",
        );
    }
    let mut bad = p.clone();
    bad.functions[0].cleanup_plans[0].span.start -= 1;
    message(
        plan::derive(&bad, &inputs(&bad, &world), &mut 0)
            .expect_err("failure span binds exact instruction"),
        "another source site",
    );
    plan::derive(&p, &inputs(&p, &world), &mut 0).expect("pristine snapshot retry");
}
#[test]
fn moved_prepared_operand_or_observed_shared_source_is_rejected_by_liveness() {
    for case in [Case::Vector, Case::Shared, Case::Downgrade] {
        let (mut p, world) = program(case);
        p.functions[0].blocks[0].instructions.insert(
            0,
            raw::Instruction {
                site_span: span(),
                result: None,
                operation: raw::Operation::DropPlace { place: 0 },
            },
        );
        p.functions[0].cleanup_plans[0].actions.retain(|a| *a != raw::CleanupAction::DropPlace(0));
        let reason = if matches!(case, Case::Downgrade) {
            "uninitialized or already moved"
        } else {
            "moved or is unavailable"
        };
        message(
            plan::derive(&p, &inputs(&p, &world), &mut 0)
                .expect_err("owning liveness rejects consumed source"),
            reason,
        );
    }
}
#[test]
fn premature_destination_is_rejected_before_any_operand_commit() {
    for case in [Case::Vector, Case::Shared, Case::Downgrade] {
        let (p, world) = program(case);
        let typed = inputs(&p, &world);
        let mut plan = plan::derive(&p, &typed, &mut 0).expect("pristine plan");
        let context = Context { program: &p, typed: &typed, function: 0 };
        let mut state = State::new(&context).expect("actual entry state");
        let destination = p.functions[0].places.len() - 1;
        state.initialized[destination] = true;
        let before = state.copy().expect("bounded private snapshot");
        instructions::execute(
            &context,
            &mut state,
            0,
            0,
            &p.functions[0].blocks[0].instructions[0],
            &mut plan,
            &mut 0,
        )
        .expect_err("unpublished result guard");
        assert_eq!(state, before, "preparation rejection must precede operand consumption");
    }
}

#[test]
fn duplicate_affine_elements_do_not_partially_consume_prepared_state() {
    let (p, world) = program(Case::Vector);
    let typed = inputs(&p, &world);
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("pristine constructor plan");
    let context = Context { program: &p, typed: &typed, function: 0 };
    let mut state = State::new(&context).expect("actual prepared parameter owners");
    let before = state.copy().expect("small private snapshot");
    let mut i = p.functions[0].blocks[0].instructions[0].clone();
    let raw::Operation::VecConstruct { elements, .. } = &mut i.operation else {
        unreachable!("fixture Vec")
    };
    elements[1] = 0;
    message(
        instructions::execute(&context, &mut state, 0, 0, &i, &mut plan, &mut 0)
            .expect_err("duplicate affine operand"),
        "moved or is unavailable",
    );
    assert_eq!(state, before, "offside candidate validation cannot partially move the source");
}
#[test]
fn exclusive_live_loan_blocks_direct_shared_observation_before_cleanup_claim() {
    let (mut p, world) = program(Case::Downgrade);
    let shared = p.functions[0].places[0].ty;
    let original_span = p.functions[0].original_span;
    p.functions[0].loans.push(raw::Loan {
        id: 0,
        referent: shared,
        access: raw::Access::Exclusive,
        origin: raw::LoanOrigin::Place(0),
        region: raw::Region { span: original_span, role: raw::RegionRole::Lexical },
    });
    p.functions[0].blocks[0].instructions.insert(
        0,
        raw::Instruction {
            site_span: span(),
            result: None,
            operation: raw::Operation::BeginLoan { loan: 0, failure: None },
        },
    );
    p.functions[0].blocks[0].instructions.push(raw::Instruction {
        site_span: span(),
        result: None,
        operation: raw::Operation::EndLoan { loan: 0 },
    });
    p.functions[0].cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(0));
    message(
        plan::derive(&p, &inputs(&p, &world), &mut 0)
            .expect_err("direct source read conflicts with exclusive loan"),
        "place access conflicts with a live overlapping loan",
    );
    // A shared loan permits the same direct observation; the exact failure
    // snapshot still ends that live loan before releasing its Shared owner.
    p.functions[0].loans[0].access = raw::Access::Shared;
    plan::derive(&p, &inputs(&p, &world), &mut 0)
        .expect("shared live loan preserves direct Shared observation");
    // With the exclusive permission ended before observation, the failure
    // snapshot must no longer claim an EndLoan for that already-ended loan.
    p.functions[0].loans[0].access = raw::Access::Exclusive;
    p.functions[0].blocks[0].instructions.swap(1, 2);
    assert_eq!(p.functions[0].cleanup_plans[0].actions.remove(0), raw::CleanupAction::EndLoan(0));
    plan::derive(&p, &inputs(&p, &world), &mut 0)
        .expect("ended exclusive permission no longer blocks direct observation");
}
#[test]
fn actual_constructor_plan_budget_exact_extra_overflow_and_pristine_retry() {
    let (p, world) = program(Case::Vector);
    let typed = inputs(&p, &world);
    let mut cost = 0;
    plan::derive(&p, &typed, &mut cost).expect("measure actual constructor owner replay");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    plan::derive(&p, &typed, &mut exact).expect("exact remaining state credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(plan::derive(&p, &typed, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(plan::derive(&p, &typed, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    plan::derive(&p, &typed, &mut retry).expect("pristine state retry");
    assert_eq!(retry, cost);
}
