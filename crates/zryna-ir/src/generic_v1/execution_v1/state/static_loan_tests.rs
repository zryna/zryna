//! Actual checked static ancestry and replacement replay; no source/execution seal.
use super::{Context, State, instructions, plan, raw, typed};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind, TypeWorld};
use typed::continuation_test_support::{Types, instruction, push, span, value};
fn static_pair() -> (TypeWorld, raw::Program) {
    let mut builder = Builder::new();
    let integer = builder.declare(Kind::I32, &[], &mut 0).expect("i32");
    let array = builder
        .declare(Kind::FixedArray { length: 2 }, &[integer], &mut 0)
        .expect("complete two-slot original type");
    let world = builder.seal(&mut 0).expect("private finite original world");
    let mut p = push(&Types::new(), None);
    p.type_keys = world.keys().map(<[u8]>::to_vec).collect();
    let f = &mut p.functions[0];
    f.parameters = vec![raw::Parameter::Value(value(0, array))];
    f.places = vec![
        raw::Place { id: 0, ty: array, span: span(), kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: 1,
            ty: integer,
            span: span(),
            kind: raw::PlaceKind::FixedArrayConstant { base: 0, index: 0 },
        },
        raw::Place {
            id: 2,
            ty: integer,
            span: span(),
            kind: raw::PlaceKind::FixedArrayConstant { base: 0, index: 1 },
        },
        raw::Place {
            id: 3,
            ty: integer,
            span: span(),
            kind: raw::PlaceKind::FixedArrayConstant { base: 0, index: 0 },
        },
    ];
    f.loans = (0..3)
        .map(|id| raw::Loan {
            id,
            referent: integer,
            access: raw::Access::Exclusive,
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
            origin: raw::LoanOrigin::Place(id + 1),
        })
        .collect();
    f.blocks[0].instructions.clear();
    f.blocks[0].terminator = raw::Terminator::Return { value: None, cleanup: 0 };
    f.cleanup_plans = vec![raw::CleanupPlan { id: 0, span: span(), actions: vec![] }];
    (world, p)
}
#[test]
fn disjoint_static_loans_and_call_arguments_preserve_exact_alias_identity() {
    let (world, p) = static_pair();
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    let c = Context { program: &p, typed: &typed, function: 0 };
    let mut state = State::new(&c).expect("complete original array");
    state.begin(&c, 0).expect("first exclusive slot");
    state.place(&c, 2, false).expect("disjoint sibling read while first borrowed");
    state.begin(&c, 1).expect("second exclusive slot is disjoint");
    state
        .call_loans(&c, &[raw::CallArgument::Loan(0), raw::CallArgument::Loan(1)])
        .expect("exact disjoint mutable arguments");
    let before = state.copy().expect("pre-overlap snapshot");
    assert!(state.begin(&c, 2).is_err(), "duplicate physical path is an alias");
    assert_eq!(state, before, "failed establishment publishes no new loan");
    assert!(
        state.call_loans(&c, &[raw::CallArgument::Loan(0), raw::CallArgument::Loan(0)]).is_err()
    );
    assert!(state.place(&c, 3, false).is_err(), "duplicate place ID cannot evade exclusive path");
    assert!(state.place(&c, 0, false).is_err(), "whole overlaps borrowed child");
    state.end(&c, 1).expect("end second");
    state.end(&c, 0).expect("end first");
    state.place(&c, 0, false).expect("whole available after exact endings");
}
fn replacement(w: &Types, access: raw::Access) -> raw::Program {
    let mut p = push(w, Some(access));
    let f = &mut p.functions[0];
    f.loans[0].referent = w.string;
    f.parameters[0] =
        raw::Parameter::Loan(raw::LoanDefinition { id: 0, referent: w.string, access });
    f.blocks[0].instructions[0] = instruction(raw::Operation::ReplaceLoan { loan: 0, value: 0 });
    f.blocks[0].terminator = raw::Terminator::Return { value: None, cleanup: 0 };
    f.cleanup_plans = vec![raw::CleanupPlan { id: 0, span: span(), actions: vec![] }];
    p
}
#[test]
fn formal_exclusive_replacement_consumes_only_prepared_rhs_and_preserves_permission() {
    let w = Types::new();
    let p = replacement(&w, raw::Access::Exclusive);
    let typed = typed::container_test_support::phase_inputs(&p, &w.world);
    plan::derive(&p, &typed, &mut 0)
        .expect("formal caller-owned old referent is replaced, then loan ends");
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("pristine retry");
    let c = Context { program: &p, typed: &typed, function: 0 };
    let mut state = State::new(&c).expect("prepared RHS and formal permission");
    instructions::execute(
        &c,
        &mut state,
        0,
        0,
        &p.functions[0].blocks[0].instructions[0],
        &mut plan,
        &mut 0,
    )
    .expect("exact nonfallible replacement commit");
    state.read_loan(&c, 0).expect("loan source remains live");
    assert!(state.value(&c, 0).is_err(), "RHS moves exactly once");
    assert_eq!(
        state.actions(&c, &mut 0).expect("postcommit actions"),
        vec![raw::CleanupAction::EndLoan(0)]
    );
}
#[test]
fn replacement_rejects_shared_or_moved_rhs_without_mutating_prepared_state() {
    let w = Types::new();
    for access in [raw::Access::Shared, raw::Access::Exclusive] {
        let p = replacement(&w, access);
        let typed = typed::container_test_support::phase_inputs(&p, &w.world);
        let c = Context { program: &p, typed: &typed, function: 0 };
        let mut state = State::new(&c).expect("actual entry state");
        if access == raw::Access::Exclusive {
            state.take_place(&c, 0).expect("RHS moved during preparation");
        }
        let before = state.copy().expect("uncommitted snapshot");
        let ordinary = push(&w, None);
        let inputs = typed::container_test_support::phase_inputs(&ordinary, &w.world);
        let mut plan = plan::derive(&ordinary, &inputs, &mut 0).expect("independent plan capacity");
        assert!(
            instructions::execute(
                &c,
                &mut state,
                0,
                0,
                &p.functions[0].blocks[0].instructions[0],
                &mut plan,
                &mut 0
            )
            .is_err()
        );
        assert_eq!(state, before, "invalid replacement cannot destroy old or consume RHS");
    }
}

#[test]
fn replacement_work_exact_extra_overflow_and_retry_leave_failure_state_pristine() {
    let w = Types::new();
    let p = replacement(&w, raw::Access::Exclusive);
    let typed = typed::container_test_support::phase_inputs(&p, &w.world);
    let c = Context { program: &p, typed: &typed, function: 0 };
    let ordinary = push(&w, None);
    let inputs = typed::container_test_support::phase_inputs(&ordinary, &w.world);
    let run = |spent: &mut usize| {
        let mut plan = plan::derive(&ordinary, &inputs, &mut 0).expect("independent owning plan");
        let mut state = State::new(&c).expect("prepared RHS and exact exclusive formal");
        let before = state.copy().expect("precommit original snapshot");
        let result = instructions::execute(
            &c,
            &mut state,
            0,
            0,
            &p.functions[0].blocks[0].instructions[0],
            &mut plan,
            spent,
        );
        if result.is_err() {
            assert_eq!(state, before, "work rejection precedes original commit");
        }
        result
    };
    let mut cost = 0;
    run(&mut cost).expect("actual complete replacement debit");
    assert!(cost > 0);
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&mut exact).expect("exact remaining credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(run(&mut spent).is_err());
        assert_eq!(spent, before, "failed debit remains exact");
    }
    let mut retry = 0;
    run(&mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
