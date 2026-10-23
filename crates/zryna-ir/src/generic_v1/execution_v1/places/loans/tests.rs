//! Actual checked metadata retention and phase resource boundaries; no execution authority.
use super::{Row, check, raw};
use crate::generic_v1::execution_v1::{
    budget, places,
    typed::continuation_test_support::{Types, push},
};
fn fixture() -> (Types, raw::Program, Vec<Row>) {
    let w = Types::new();
    let mut p = push(&w, None);
    let f = &mut p.functions[0];
    f.loans = (0..11)
        .map(|id| raw::Loan {
            id,
            referent: w.vector,
            access: raw::Access::Shared,
            origin: raw::LoanOrigin::Place(0),
            region: raw::Region { span: f.original_span, role: raw::RegionRole::Lexical },
        })
        .collect();
    let values = [raw::ValueType::Stored(w.vector), raw::ValueType::Stored(w.string)];
    let table = places::check(&p.functions[0], &values, &w.world, &mut 0)
        .expect("genuine private parameter/place/type kernel");
    (w, p, table.rows)
}
#[test]
fn extra_retained_loan_row_slot_is_paid_before_exact_extra_overflow_and_retry() {
    let (w, p, rows) = fixture();
    let f = &p.functions[0];
    // Five existing checked metadata visits plus one new retained referent slot.
    // Ordinary parameter visits remain independently prepaid on this phase.
    let cost = 6 * f.loans.len() + f.parameters.len();
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    assert_eq!(check(f, &rows, &w.world, &mut exact).expect("exact retained slots").len(), 11);
    assert_eq!(exact, limit, "every retained slot paid before row reserve/write");
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(check(f, &rows, &w.world, &mut spent).is_err());
        assert_eq!(spent, before, "failed prepay publishes no completed debit");
    }
    let mut retry = 0;
    check(f, &rows, &w.world, &mut retry).expect("pristine metadata retry");
    assert_eq!(retry, cost);
}
#[test]
fn complete_place_and_retained_loan_phase_has_exact_extra_overflow_and_retry() {
    let (w, p, _) = fixture();
    let values = [raw::ValueType::Stored(w.vector), raw::ValueType::Stored(w.string)];
    let run = |spent: &mut usize| places::check(&p.functions[0], &values, &w.world, spent);
    let mut cost = 0;
    run(&mut cost).expect("actual complete owning place phase");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    run(&mut exact).expect("exact full phase including retained reference resolution");
    assert_eq!(exact, limit);
    for mut spent in [limit - cost + 1, usize::MAX] {
        let before = spent;
        assert!(run(&mut spent).is_err());
        assert_eq!(spent, before);
    }
    let mut retry = 0;
    run(&mut retry).expect("pristine complete phase retry");
    assert_eq!(retry, cost);
}
