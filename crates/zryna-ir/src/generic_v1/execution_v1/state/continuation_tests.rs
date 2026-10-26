//! Actual owner replay and precommit controls; private phase inputs issue no execution seal.
use super::{Context, State, instructions, plan, raw, typed};
use typed::continuation_test_support::{Types, push, upgrade};
fn derive(w: &Types, p: &raw::Program) -> Result<plan::Plan, super::Failure> {
    let t = typed::container_test_support::phase_inputs(p, &w.world);
    plan::derive(p, &t, &mut 0)
}
#[test]
fn push_failure_owns_vector_and_rhs_success_retains_only_vector() {
    let w = Types::new();
    for access in [None, Some(raw::Access::Exclusive)] {
        derive(&w, &push(&w, access))
            .expect("precommit cleanup includes prepared RHS; success consumes only RHS");
    }
    for missing in [0, 1] {
        let mut p = push(&w, None);
        p.functions[0].cleanup_plans[0].actions.remove(missing);
        assert!(derive(&w, &p).is_err(), "failure may omit neither vector nor RHS");
    }
    let mut span = push(&w, None);
    span.functions[0].cleanup_plans[0].span.start += 1;
    assert!(derive(&w, &span).is_err(), "failure must retain exact operation site");
}
#[test]
fn push_rechecks_moved_rhs_and_vector_and_live_aliases_without_partial_commit() {
    let w = Types::new();
    let p = push(&w, None);
    let t = typed::container_test_support::phase_inputs(&p, &w.world);
    let mut plan = derive(&w, &p).expect("pristine plan before hostile replay");
    let c = Context { program: &p, typed: &t, function: 0 };
    for moved in [0, 1] {
        let mut state = State::new(&c).expect("actual entry owners");
        state.take_place(&c, moved).expect("RHS preparation moved one live owner");
        let before = state.copy().expect("pre-instruction snapshot");
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
        assert_eq!(state, before, "reject before committing any original owner");
    }
    let mut alias = push(&w, None);
    let alias_span = alias.functions[0].original_span;
    alias.functions[0].loans.push(raw::Loan {
        id: 0,
        referent: w.vector,
        access: raw::Access::Shared,
        region: raw::Region { span: alias_span, role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Place(0),
    });
    alias.functions[0].blocks[0].instructions.insert(
        0,
        typed::continuation_test_support::instruction(raw::Operation::BeginLoan {
            loan: 0,
            failure: None,
        }),
    );
    alias.functions[0].cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(0));
    assert!(derive(&w, &alias).is_err(), "post-RHS direct target conflicts with live alias");
    let mut loan = push(&w, Some(raw::Access::Shared));
    assert!(derive(&w, &loan).is_err(), "shared target never grants mutation");
    loan.functions[0].loans[0].access = raw::Access::Exclusive;
    loan.functions[0].parameters[0] = raw::Parameter::Loan(raw::LoanDefinition {
        id: 0,
        referent: w.vector,
        access: raw::Access::Exclusive,
    });
    derive(&w, &loan).expect("exact exclusive retry");
}
#[test]
fn upgrade_failure_excludes_unpublished_result_success_cleanup_precedes_retained_weak() {
    let w = Types::new();
    let p = upgrade(&w);
    derive(&w, &p).expect("both actual edges retain Weak; success alone owns new Shared");
    let mut unpublished = upgrade(&w);
    unpublished.functions[0].cleanup_plans[0].actions.insert(0, raw::CleanupAction::DropPlace(1));
    assert!(derive(&w, &unpublished).is_err(), "count failure cannot own unpublished Shared");
    let mut order = upgrade(&w);
    order.functions[0].cleanup_plans[1].actions.reverse();
    assert!(derive(&w, &order).is_err(), "success binding cleanup is newer than source Weak");
    let mut expired = upgrade(&w);
    expired.functions[0].cleanup_plans[2].actions.push(raw::CleanupAction::DropPlace(1));
    assert!(derive(&w, &expired).is_err(), "expired edge never publishes success binding");
}
#[test]
fn upgrade_live_success_owner_must_be_cleaned_before_shared_join() {
    let w = Types::new();
    let mut p = upgrade(&w);
    p.functions[0].blocks[1].terminator =
        raw::Terminator::Jump(raw::Edge { target: 3, arguments: vec![] });
    p.functions[0].blocks[2].terminator =
        raw::Terminator::Jump(raw::Edge { target: 3, arguments: vec![] });
    p.functions[0].blocks.push(raw::Block {
        id: 3,
        value_parameters: vec![],
        instructions: vec![],
        terminator: raw::Terminator::Return { value: None, cleanup: 1 },
    });
    p.functions[0].cleanup_plans.remove(2);
    assert!(derive(&w, &p).is_err(), "a path-only live owner cannot disappear at a join");
    p.functions[0].blocks[1].instructions.push(typed::continuation_test_support::instruction(
        raw::Operation::DropPlace { place: 1 },
    ));
    p.functions[0].cleanup_plans[1].actions = vec![raw::CleanupAction::DropPlace(0)];
    derive(&w, &p).expect("both real branch states now agree at join");
}

mod nested;
