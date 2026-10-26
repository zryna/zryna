//! Loop entry distinguishes retained formal permissions from lexical authorities.
use super::loop_tests::fixture::input;
use super::{plan, raw, typed};
fn derive(
    p: &raw::Program,
    world: &crate::generic_v1::execution_v1::type_world::TypeWorld,
) -> bool {
    let checked = typed::container_test_support::phase_inputs(p, world);
    plan::derive(p, &checked, &mut 0).is_ok()
}
#[test]
fn live_lexical_loan_cannot_cross_first_or_repeated_header_entry() {
    let (mut p, world) = input(false);
    let f = &mut p.functions[0];
    f.loans.push(raw::Loan {
        id: 0,
        referent: f.places[0].ty,
        access: raw::Access::Shared,
        region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Place(0),
    });
    f.blocks[0].instructions.push(typed::continuation_test_support::instruction(
        raw::Operation::BeginLoan { loan: 0, failure: None },
    ));
    f.cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(0));
    assert!(!derive(&p, &world), "equal loan sets alone do not authorize lexical loop escape");
}
#[test]
fn unchanged_formal_call_permission_is_not_a_stored_lexical_edge_argument() {
    let (mut p, world) = input(false);
    let f = &mut p.functions[0];
    let referent = f.places[0].ty;
    f.parameters.insert(
        0,
        raw::Parameter::Loan(raw::LoanDefinition { id: 0, referent, access: raw::Access::Shared }),
    );
    f.places[0].kind = raw::PlaceKind::Parameter(1);
    f.loans.push(raw::Loan {
        id: 0,
        referent,
        access: raw::Access::Shared,
        region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Formal(0),
    });
    f.cleanup_plans[0].actions.insert(0, raw::CleanupAction::EndLoan(0));
    assert!(derive(&p, &world), "formal permission remains exact and nonescaping");
}
