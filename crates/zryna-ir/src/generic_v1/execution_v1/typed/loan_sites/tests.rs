//! Private owning controls: raw rows here issue no source proof or execution authority.
use super::{instruction, raw, role, terminator};
use zryna_source::UntrustedSpan;

fn span(file: u32, start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file, start, end }
}
fn function() -> raw::Function {
    raw::Function {
        key: vec![],
        original_span: span(0, 0, 100),
        executable_export: None,
        parameters: vec![],
        result_type: raw::ValueType::Unit,
        places: vec![],
        loans: vec![raw::Loan {
            id: 0,
            referent: 1,
            access: raw::Access::Shared,
            region: raw::Region { span: span(0, 10, 20), role: raw::RegionRole::Lexical },
            origin: raw::LoanOrigin::Place(0),
        }],
        blocks: vec![],
        cleanup_plans: vec![raw::CleanupPlan { id: 0, span: span(0, 0, 100), actions: vec![] }],
    }
}
fn at(operation: raw::Operation, site_span: UntrustedSpan) -> raw::Instruction {
    raw::Instruction { site_span, result: None, operation }
}
fn uses() -> Vec<raw::Operation> {
    vec![
        raw::Operation::Clone {
            source: raw::PlaceOrLoan::Loan(0),
            outer_failure: None,
            prefix_failure: None,
        },
        raw::Operation::IndexCopy { base: raw::PlaceOrLoan::Loan(0), index: 3, failure: 0 },
        raw::Operation::BeginLoan { loan: 0, failure: None },
        raw::Operation::WeakDowngrade { shared: raw::PlaceOrLoan::Loan(0), failure: 0 },
        raw::Operation::VecPush { vector: raw::PlaceOrLoan::Loan(0), value: 3, failure: 0 },
        raw::Operation::EndLoan { loan: 0 },
        raw::Operation::ReadLoan { loan: 0 },
        raw::Operation::SourceCall {
            module: 0,
            function: 0,
            arguments: vec![raw::CallArgument::Loan(0)],
            failure: None,
        },
        raw::Operation::ClosedGenericCall {
            instance: 0,
            arguments: vec![raw::CallArgument::Loan(0)],
            failure: None,
        },
    ]
}
#[test]
fn every_admitted_instruction_loan_role_requires_exact_site_containment() {
    let f = function();
    for op in uses() {
        instruction(&f, &at(op.clone(), span(0, 10, 20)), &mut 0).expect("exact lexical bounds");
        for site in [span(0, 9, 20), span(0, 10, 21), span(1, 10, 20), span(0, 18, 17)] {
            assert!(instruction(&f, &at(op.clone(), site), &mut 0).is_err());
        }
    }
}
#[test]
fn foreign_loan_ids_and_unfinished_roles_do_not_receive_site_authority() {
    let mut f = function();
    f.loans[0].id = 1;
    assert!(
        instruction(&f, &at(raw::Operation::ReadLoan { loan: 0 }, span(0, 12, 13)), &mut 0)
            .is_err()
    );
    let f = function();
    assert!(
        instruction(&f, &at(raw::Operation::ReadLoan { loan: 99 }, span(0, 12, 13)), &mut 0)
            .is_err()
    );
    for role in [raw::RegionRole::SelectedMatchArm, raw::RegionRole::TransientIndexedAccess] {
        let mut f = function();
        f.loans[0].region.role = role;
        assert!(super::role(&f.loans[0]).is_err());
        assert!(
            instruction(&f, &at(raw::Operation::ReadLoan { loan: 0 }, span(0, 12, 13)), &mut 0)
                .is_err()
        );
    }
    for origin in [
        raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(0), index: 3 },
        raw::LoanOrigin::StaticProjection {
            parent: 0,
            projection: raw::StaticProjection::EnumPayload(1),
        },
    ] {
        let mut f = function();
        f.loans[0].origin = origin;
        assert!(role(&f.loans[0]).is_err());
        assert!(
            instruction(
                &f,
                &at(raw::Operation::BeginLoan { loan: 0, failure: None }, span(0, 12, 13)),
                &mut 0
            )
            .is_err()
        );
    }
    assert!(
        instruction(
            &f,
            &at(raw::Operation::ReplaceLoan { loan: 0, value: 3 }, span(0, 9, 13)),
            &mut 0
        )
        .is_err()
    );
}
#[test]
fn terminator_loan_use_and_cleanup_require_a_real_source_bound_use_fact() {
    let mut f = function();
    let t = raw::Terminator::ClosedEnumMatch {
        ty: 1,
        scrutinee: raw::PlaceOrLoan::Loan(0),
        mode: raw::MatchMode::Value,
        arms: vec![],
    };
    assert!(terminator(&f, &t, &mut 0).is_err());
    for t in [
        raw::Terminator::Return { value: None, cleanup: 0 },
        raw::Terminator::Trap { identity: raw::TrapIdentity::Bounds, cleanup: 0 },
    ] {
        terminator(&f, &t, &mut 0).expect("no loan operand or cleanup");
        f.cleanup_plans[0].span = f.loans[0].region.span;
        f.cleanup_plans[0].actions.push(raw::CleanupAction::EndLoan(0));
        assert!(terminator(&f, &t, &mut 0).is_err());
        f.cleanup_plans[0].actions.clear();
    }
}
#[test]
fn site_and_call_scan_budget_is_atomic_at_exact_limit_overflow_and_retry() {
    let f = function();
    let i = at(
        raw::Operation::SourceCall {
            module: 0,
            function: 0,
            arguments: vec![raw::CallArgument::Value(3), raw::CallArgument::Loan(0)],
            failure: None,
        },
        span(0, 12, 13),
    );
    let mut cost = 0;
    instruction(&f, &i, &mut cost).expect("actual call loan visit");
    let limit = super::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    instruction(&f, &i, &mut exact).expect("exact remaining credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(instruction(&f, &i, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(instruction(&f, &i, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    instruction(&f, &i, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}

#[test]
fn formal_begin_and_fallible_preparation_are_concrete_unfinished_rows() {
    let mut f = function();
    f.loans[0].origin = raw::LoanOrigin::Formal(0);
    assert!(
        instruction(
            &f,
            &at(raw::Operation::BeginLoan { loan: 0, failure: None }, span(0, 12, 13)),
            &mut 0
        )
        .is_err()
    );
    let f = function();
    assert!(
        instruction(
            &f,
            &at(raw::Operation::BeginLoan { loan: 0, failure: Some(0) }, span(0, 12, 13)),
            &mut 0
        )
        .is_err()
    );
}
#[test]
fn terminator_scan_budget_precedes_visits_and_failure_is_atomic() {
    let f = function();
    let t = raw::Terminator::Return { value: None, cleanup: 0 };
    let mut cost = 0;
    terminator(&f, &t, &mut cost).expect("measure empty cleanup scan");
    let limit = super::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    terminator(&f, &t, &mut exact).expect("exact remaining credit");
    assert_eq!(exact, limit);
    let mut extra = limit - cost + 1;
    let before = extra;
    assert!(terminator(&f, &t, &mut extra).is_err());
    assert_eq!(extra, before);
    let mut overflow = usize::MAX;
    assert!(terminator(&f, &t, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    terminator(&f, &t, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}

#[test]
fn replacement_and_failure_cleanup_require_the_exact_instruction_loan_region() {
    let f = function();
    instruction(
        &f,
        &at(raw::Operation::ReplaceLoan { loan: 0, value: 3 }, span(0, 12, 13)),
        &mut 0,
    )
    .expect("replacement has an authentic in-region loan use");
    let mut f = function();
    f.cleanup_plans[0].actions = vec![raw::CleanupAction::EndLoan(0)];
    let operation = raw::Operation::StringFromUtf8 { bytes: vec![65], failure: 0 };
    instruction(&f, &at(operation.clone(), span(0, 12, 13)), &mut 0)
        .expect("fallible instruction can end its live in-region loan");
    for site in [span(0, 9, 13), span(0, 12, 21), span(1, 12, 13)] {
        let mut spent = 7;
        assert!(instruction(&f, &at(operation.clone(), site), &mut spent).is_err());
        assert_eq!(spent, 7, "failed site authentication is atomic");
    }
    f.cleanup_plans[0].actions = vec![raw::CleanupAction::EndLoan(99)];
    assert!(instruction(&f, &at(operation, span(0, 12, 13)), &mut 0).is_err());
}
