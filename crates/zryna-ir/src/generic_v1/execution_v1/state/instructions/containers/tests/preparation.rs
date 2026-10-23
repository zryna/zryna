//! Runs the actual preparation gate before final publication; no source or execution seal.
use super::super::{Context, State, execute, plan, raw};
use crate::generic_v1::execution_v1::{
    type_world::{Builder, Kind, TypeWorld},
    typed,
};
use zryna_source::UntrustedSpan;

#[derive(Clone, Copy)]
enum Case {
    EmptyVec,
    CopyShared,
    FormalLoanDowngrade,
}

#[test]
fn empty_vec_prepares_fallible_result_owner_slot_before_commit() {
    check(Case::EmptyVec);
}
#[test]
fn copy_payload_shared_prepares_fallible_result_owner_slot_before_commit() {
    check(Case::CopyShared);
}
#[test]
fn zero_owned_root_formal_loan_downgrade_prepares_result_owner_slot() {
    check(Case::FormalLoanDowngrade);
}

fn check(case: Case) {
    let (p, world) = fixture(case);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("actual independent owner replay");
    let context = Context { program: &p, typed: &typed, function: 0 };
    // Replay copies incoming state. Start with that exact logical copy, now retaining
    // every authority-bounded owner slot even when the live owner list is empty.
    let mut state = State::new(&context).expect("entry state").copy().expect("incoming copy");
    assert!(state.owners.is_empty(), "no prepared operand owns an affine root");
    assert!(
        state.owners.capacity() >= state.initialized.len(),
        "incoming copy retains prepaid owner capacity"
    );
    let i = &p.functions[0].blocks[0].instructions[0];
    let mut work = 0;
    execute(&context, &mut state, 0, i, &mut plan, &mut work)
        .expect("actual container preparation");
    assert!(state.owners.is_empty(), "result remains unpublished before final publish");
    assert!(state.owners.capacity() >= 1, "fallible spare owner reservation must precede commit");
    let capacity = state.owners.capacity();
    state
        .publish(&context, i.result.as_ref().expect("result"), None)
        .expect("publish prepared result");
    assert_eq!(state.owners.len(), 1);
    assert_eq!(state.owners.capacity(), capacity, "publication cannot acquire new capacity");
    if matches!(case, Case::FormalLoanDowngrade) {
        assert!(state.live_loans[0], "downgrade retains its observed formal source loan");
    }
}
fn definition(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
fn fixture(case: Case) -> (raw::Program, TypeWorld) {
    let mut builder = Builder::new();
    let integer = builder.declare(Kind::I32, &[], &mut 0).expect("i32");
    let vector = builder.declare(Kind::Vec, &[integer], &mut 0).expect("Vec<i32>");
    let shared = builder.declare(Kind::Shared, &[integer], &mut 0).expect("Shared<i32>");
    let weak = builder.declare(Kind::Weak, &[integer], &mut 0).expect("Weak<i32>");
    let world = builder.seal(&mut 0).expect("checked private capability graph");
    let span = UntrustedSpan { file: 0, start: 0, end: 10 };
    let (parameters, loans, result, operation) = match case {
        Case::EmptyVec => (
            vec![],
            vec![],
            definition(0, vector),
            raw::Operation::VecConstruct { ty: vector, elements: vec![], failure: 0 },
        ),
        Case::CopyShared => (
            vec![raw::Parameter::Value(definition(0, integer))],
            vec![],
            definition(1, shared),
            raw::Operation::SharedConstruct { ty: shared, value: 0, failure: 0 },
        ),
        Case::FormalLoanDowngrade => (
            vec![raw::Parameter::Loan(raw::LoanDefinition {
                id: 0,
                referent: shared,
                access: raw::Access::Shared,
            })],
            vec![raw::Loan {
                id: 0,
                referent: shared,
                access: raw::Access::Shared,
                region: raw::Region { span, role: raw::RegionRole::Lexical },
                origin: raw::LoanOrigin::Formal(0),
            }],
            definition(0, weak),
            raw::Operation::WeakDowngrade { shared: raw::PlaceOrLoan::Loan(0), failure: 0 },
        ),
    };
    let actions = if loans.is_empty() { vec![] } else { vec![raw::CleanupAction::EndLoan(0)] };
    let places = vec![raw::Place {
        id: 0,
        ty: match result.ty {
            raw::ValueType::Stored(ty) => ty,
            raw::ValueType::Unit => unreachable!(),
        },
        span,
        kind: raw::PlaceKind::Temporary(result.id),
    }];
    let result_type = result.ty;
    let return_value = result.id;
    let f = raw::Function {
        key: vec![],
        original_span: span,
        executable_export: None,
        parameters,
        result_type,
        places,
        loans,
        blocks: vec![raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![raw::Instruction {
                site_span: span,
                result: Some(result),
                operation,
            }],
            terminator: raw::Terminator::Return { value: Some(return_value), cleanup: 1 },
        }],
        cleanup_plans: vec![
            raw::CleanupPlan { id: 0, span, actions: actions.clone() },
            raw::CleanupPlan { id: 1, span, actions },
        ],
    };
    (
        raw::Program {
            language_contract: raw::CONTRACT.into(),
            modules: vec![],
            declarations: vec![],
            type_keys: world.keys().map(<[u8]>::to_vec).collect(),
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            entry_module: 0,
            functions: vec![f],
        },
        world,
    )
}
