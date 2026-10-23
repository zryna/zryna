//! Owning prewalk controls do not issue state, source, or execution authority.
use super::{budget, census, precharge, raw};
fn function() -> raw::Function {
    raw::Function {
        key: vec![],
        original_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        executable_export: None,
        parameters: vec![],
        result_type: raw::ValueType::Unit,
        places: vec![],
        loans: vec![],
        cleanup_plans: vec![],
        blocks: (0..8)
            .map(|id| raw::Block {
                id,
                value_parameters: vec![],
                instructions: vec![],
                terminator: raw::Terminator::Return { value: None, cleanup: 0 },
            })
            .collect(),
    }
}
#[test]
fn every_empty_block_header_pass_is_paid_before_future_state_credit() {
    let f = function();
    let mut census_cost = 0;
    census(&f, &mut census_cost).expect("actual three metadata walks");
    assert!(
        census_cost >= 3 * f.blocks.len(),
        "empty instructions/edges provide no header payment"
    );
    let mut retention_cost = 0;
    precharge(&f, 0, &mut retention_cost).expect("complete future state work");
    assert!(
        retention_cost > census_cost,
        "future footprint credit stays distinct from prewalk payment"
    );
}
#[test]
fn actual_metadata_census_exact_remaining_credit_first_extra_overflow_and_retry() {
    let f = function();
    let mut cost = 0;
    census(&f, &mut cost).expect("actual census cost");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    census(&f, &mut exact).expect("exact metadata credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(census(&f, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(census(&f, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    census(&f, &mut retry).expect("pristine metadata retry");
    assert_eq!(retry, cost);
}
#[test]
fn future_state_preflight_preserves_caller_when_footprint_cannot_be_prepaid() {
    let f = function();
    let mut cost = 0;
    precharge(&f, 0, &mut cost).expect("actual combined preflight cost");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    precharge(&f, 0, &mut exact).expect("exact combined credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(precharge(&f, 0, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(precharge(&f, 0, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    precharge(&f, 0, &mut retry).expect("pristine combined retry");
    assert_eq!(retry, cost);
}
