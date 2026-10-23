//! Pure source-row inventory/lookup controls; no `SourceProof` or execution seal is issued.
use super::{Upgrades, raw};
use crate::generic_v1::execution_v1::typed::continuation_test_support::{Types, span, upgrade};
#[test]
fn exact_source_row_retains_original_site_and_rejects_foreign_use_fields() {
    let mut expected = upgrade(&Types::new());
    let rows = Upgrades::new(&expected, &mut 0).expect("expected-source rows");
    expected.functions[0].cleanup_plans[0].span.start += 1;
    assert_eq!(
        rows.site(0, 0, raw::PlaceOrLoan::Place(0), 0, &mut 0).expect("retained original use"),
        span()
    );
    for (function, block, weak, failure) in [
        (1, 0, raw::PlaceOrLoan::Place(0), 0),
        (0, 1, raw::PlaceOrLoan::Place(0), 0),
        (0, 0, raw::PlaceOrLoan::Loan(0), 0),
        (0, 0, raw::PlaceOrLoan::Place(0), 1),
    ] {
        assert!(rows.site(function, block, weak, failure, &mut 0).is_err());
    }
}
#[test]
fn actual_source_row_reservation_and_lookup_have_exact_extra_overflow_and_retry() {
    let expected = upgrade(&Types::new());
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut cost = 0;
    let rows = Upgrades::new(&expected, &mut cost).expect("actual source inventory");
    let mut exact = limit - cost;
    Upgrades::new(&expected, &mut exact).expect("exact inventory credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(Upgrades::new(&expected, &mut spent).is_err());
        assert_eq!(spent, before);
    }
    let mut lookup = 0;
    rows.site(0, 0, raw::PlaceOrLoan::Place(0), 0, &mut lookup).expect("actual row lookup");
    let mut exact = limit - lookup;
    rows.site(0, 0, raw::PlaceOrLoan::Place(0), 0, &mut exact).expect("exact lookup credit");
    assert_eq!(exact, limit);
    for mut spent in [limit - lookup + 1, usize::MAX] {
        let before = spent;
        assert!(rows.site(0, 0, raw::PlaceOrLoan::Place(0), 0, &mut spent).is_err());
        assert_eq!(spent, before);
    }
    let mut retry = 0;
    Upgrades::new(&expected, &mut retry).expect("pristine inventory retry");
    assert_eq!(retry, cost);
}
