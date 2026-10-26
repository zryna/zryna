//! Pure source-bound site validation; supplying this private test span issues no authority.
use super::{check, raw};
use crate::generic_v1::execution_v1::typed::continuation_test_support::{Types, span, upgrade};
fn function() -> raw::Function {
    let w = Types::new();
    let mut f = upgrade(&w).functions.remove(0);
    let site = span();
    f.loans.push(raw::Loan {
        id: 0,
        referent: w.weak,
        access: raw::Access::Shared,
        region: raw::Region { span: site, role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Formal(0),
    });
    f.cleanup_plans[0].actions = vec![raw::CleanupAction::EndLoan(0)];
    let raw::Terminator::WeakUpgradeBranch { weak, .. } = &mut f.blocks[0].terminator else {
        unreachable!()
    };
    *weak = raw::PlaceOrLoan::Loan(0);
    f
}
#[test]
fn retained_upgrade_site_checks_exact_loan_region_and_cleanup_source_span() {
    let f = function();
    check(&f, &f.blocks[0].terminator, span(), &mut 0)
        .expect("exact original site inside loan region");
    for delta in [0, 1, 2, 3] {
        let mut wrong = function();
        match delta {
            0 => wrong.loans[0].region.span.start += 1,
            1 => wrong.loans[0].region.span.end -= 1,
            2 => wrong.loans[0].region.span.file += 1,
            _ => wrong.cleanup_plans[0].span.start += 1,
        }
        assert!(check(&wrong, &wrong.blocks[0].terminator, span(), &mut 0).is_err());
    }
    let mut foreign = function();
    foreign.cleanup_plans[0].actions.push(raw::CleanupAction::EndLoan(1));
    assert!(check(&foreign, &foreign.blocks[0].terminator, span(), &mut 0).is_err());
}
#[test]
fn upgrade_site_scan_is_exact_extra_overflow_atomic_and_retryable() {
    let f = function();
    let mut cost = 0;
    check(&f, &f.blocks[0].terminator, span(), &mut cost).expect("actual site and cleanup scan");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    check(&f, &f.blocks[0].terminator, span(), &mut exact).expect("exact site scan credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(check(&f, &f.blocks[0].terminator, span(), &mut spent).is_err());
        assert_eq!(spent, before);
    }
    let mut retry = 0;
    check(&f, &f.blocks[0].terminator, span(), &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
