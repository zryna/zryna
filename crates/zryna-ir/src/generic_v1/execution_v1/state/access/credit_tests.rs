//! Real instruction replay distinguishes active alias work from unused future loan rows.
use super::super::{Context, Failure, State, instructions, plan, raw, typed};
use typed::continuation_test_support::{Types, instruction, push, span, value};
fn fixture(w: &Types, active: u32) -> raw::Program {
    let mut p = push(w, None);
    let f = &mut p.functions[0];
    f.parameters.push(raw::Parameter::Value(value(2, w.integer)));
    f.places.push(raw::Place {
        id: 2,
        ty: w.integer,
        span: span(),
        kind: raw::PlaceKind::Parameter(2),
    });
    f.loans = (0..11)
        .map(|id| raw::Loan {
            id,
            referent: w.integer,
            access: raw::Access::Shared,
            origin: raw::LoanOrigin::Place(2),
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        })
        .collect();
    for id in (0..active).rev() {
        f.blocks[0]
            .instructions
            .insert(0, instruction(raw::Operation::BeginLoan { loan: id, failure: None }));
    }
    for id in (0..active).rev() {
        f.blocks[0].instructions.push(instruction(raw::Operation::EndLoan { loan: id }));
    }
    for id in 0..active {
        f.cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(id));
    }
    p
}
fn replay(active: u32, spent: &mut usize) -> Result<(), Failure> {
    let w = Types::new();
    let p = fixture(&w, active);
    let t = typed::container_test_support::phase_inputs(&p, &w.world);
    let mut plan = plan::derive(&p, &t, &mut 0).expect("whole actual precommit/exit plan");
    let c = Context { program: &p, typed: &t, function: 0 };
    let mut state = State::new(&c).expect("actual ordinary parameter owners");
    for ii in 0..usize::try_from(active).expect("bounded prepared loans") {
        instructions::execute(
            &c,
            &mut state,
            0,
            ii,
            &p.functions[0].blocks[0].instructions[ii],
            &mut plan,
            &mut 0,
        )
        .expect("real preceding BeginLoan instruction");
    }
    let ii = usize::try_from(active).expect("actual push index");
    let before = state.copy().expect("complete precommit prepared state");
    let result = instructions::execute(
        &c,
        &mut state,
        0,
        ii,
        &p.functions[0].blocks[0].instructions[ii],
        &mut plan,
        spent,
    );
    if result.is_err() {
        assert_eq!(state, before, "credit failure precedes original commit");
    }
    result
}
#[test]
fn active_alias_visits_are_paid_separately_from_future_rows_and_snapshot_entries() {
    let mut inactive = 0;
    replay(0, &mut inactive).expect("eleven future loan rows remain inactive");
    let mut active = 0;
    replay(1, &mut active).expect("one unrelated shared alias is actually visited");
    // One extra candidate-order read, cleanup action and claim comparison pay
    // three units. Alias traversal must carry additional work of its own.
    assert!(active > inactive + 3, "active alias visits need independent path payment");
    let mut pristine = 0;
    replay(0, &mut pristine).expect("future rows must not behave as live aliases on retry");
    assert_eq!(pristine, inactive);
}
#[test]
fn two_live_aliases_have_exact_extra_overflow_and_pristine_instruction_credit() {
    let mut cost = 0;
    replay(2, &mut cost).expect("actual two-live-alias push");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    replay(2, &mut exact).expect("exact remaining instruction credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(replay(2, &mut spent).is_err());
        assert_eq!(spent, before, "failed full instruction debit is atomic");
    }
    let mut retry = 0;
    replay(2, &mut retry).expect("pristine two-live-alias retry");
    assert_eq!(retry, cost);
}
