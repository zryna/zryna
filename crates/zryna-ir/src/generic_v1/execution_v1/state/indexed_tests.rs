//! Owning state kernel controls: no manufactured positive whole-source or execution seal.
use super::raw;
use crate::generic_v1::execution_v1::typed::continuation_test_support::instruction;
mod fixture;
use fixture::{across_edge, derive, fixture};
#[test]
fn bounds_failure_excludes_new_child_and_success_consumes_only_prepared_rhs() {
    for role in [raw::RegionRole::Lexical, raw::RegionRole::TransientIndexedAccess] {
        let (w, p) = fixture(role);
        derive(&w, &p).expect("exact pre-Begin owners then complete RHS replacement and child end");
        let mut claimed_child = p.clone();
        claimed_child.functions[0].cleanup_plans[0]
            .actions
            .insert(0, raw::CleanupAction::EndLoan(0));
        assert!(derive(&w, &claimed_child).is_err(), "bounds failure cannot end unpublished child");
        let mut missing_rhs = p.clone();
        missing_rhs.functions[0].cleanup_plans[0].actions.remove(0);
        assert!(derive(&w, &missing_rhs).is_err(), "bounds failure retains prepared entry owners");
    }
}
#[test]
fn live_exclusive_indexed_receiver_blocks_rhs_whole_container_borrows() {
    for access in [raw::Access::Shared, raw::Access::Exclusive] {
        let (w, mut p) = fixture(raw::RegionRole::TransientIndexedAccess);
        let f = &mut p.functions[0];
        f.loans.push(raw::Loan {
            id: 1,
            referent: w.vector,
            access,
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
            origin: raw::LoanOrigin::Place(0),
        });
        f.blocks[0]
            .instructions
            .insert(1, instruction(raw::Operation::BeginLoan { loan: 1, failure: None }));
        assert!(
            derive(&w, &p).is_err(),
            "prepared indexed target retains whole-container exclusive region during RHS"
        );
    }
}
#[test]
fn transient_identity_crosses_acyclic_edge_while_lexical_and_unfinished_returns_reject() {
    let (w, mut p) = fixture(raw::RegionRole::TransientIndexedAccess);
    across_edge(&mut p);
    derive(&w, &p)
        .expect("same index/base/loan identity and issuance order survive acyclic continuation");
    p.functions[0].loans[0].region.role = raw::RegionRole::Lexical;
    assert!(
        derive(&w, &p).is_err(),
        "lexical indexed loan cannot silently become continuation authority"
    );
    let (w, mut p) = fixture(raw::RegionRole::TransientIndexedAccess);
    p.functions[0].blocks[0].instructions.truncate(1);
    assert!(derive(&w, &p).is_err(), "return requires completed indexed operation");
}
#[test]
fn transient_loan_cannot_reenter_loop_header() {
    let (w, mut p) = fixture(raw::RegionRole::TransientIndexedAccess);
    across_edge(&mut p);
    let f = &mut p.functions[0];
    f.blocks[1].instructions.clear();
    f.blocks[1].terminator = raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![] });
    f.cleanup_plans.truncate(1);
    assert!(
        derive(&w, &p).is_err(),
        "cyclic continuation cannot retain unfinished checked address"
    );
}

mod regions;

mod calls;
