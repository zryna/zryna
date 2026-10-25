//! Pure owning census controls: no source proof or execution seal is constructed.
use super::{Failure, budget, census, raw};
fn program(parameters: usize, block_parameters: usize) -> raw::Program {
    let span = zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 };
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: vec![],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![raw::Function {
            key: vec![0x41],
            original_span: span,
            executable_export: None,
            parameters: (0..parameters)
                .map(|id| {
                    raw::Parameter::Loan(raw::LoanDefinition {
                        id: u32::try_from(id).expect("bounded test parameter ID"),
                        referent: 0,
                        access: raw::Access::Shared,
                    })
                })
                .collect(),
            result_type: raw::ValueType::Unit,
            places: vec![],
            loans: vec![],
            blocks: if block_parameters == 0 {
                vec![]
            } else {
                vec![raw::Block {
                    id: 0,
                    value_parameters: (0..block_parameters)
                        .map(|id| raw::ValueDefinition {
                            id: u32::try_from(id).expect("bounded test parameter ID"),
                            ty: raw::ValueType::Unit,
                        })
                        .collect(),
                    instructions: vec![],
                    terminator: raw::Terminator::Return { value: None, cleanup: 0 },
                }]
            },
            cleanup_plans: vec![],
        }],
    }
}
#[test]
fn all_loan_parameters_pay_both_independent_census_visits() {
    let p = program(256, 0);
    let mut cost = 0;
    census(&p, &mut cost).expect("private signature inventory census");
    assert!(
        cost >= 2 * p.functions[0].parameters.len(),
        "loan parameters cannot be paid by later ordinary-value retention"
    );
}
#[test]
fn exhausted_block_header_credit_rejects_before_inspecting_malformed_metadata() {
    let p = program(0, 257);
    let mut spent = budget::MAX_OWNER_UNITS - 4;
    let failure = census(&p, &mut spent)
        .expect_err("header credit is exhausted before the malformed block is visited");
    let Failure::Diagnostics(diagnostics) = failure else {
        panic!("owning resource diagnostic expected")
    };
    assert!(
        diagnostics.iter().any(|d| d.message == "full-profile ownership verification work ceiling"),
        "a later block-parameter limit is not proof of prepaying the header"
    );
}
#[test]
fn census_exact_remaining_limit_first_extra_overflow_and_retry_are_atomic() {
    let p = program(256, 0);
    let mut cost = 0;
    census(&p, &mut cost).expect("measure complete census");
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    census(&p, &mut exact).expect("exact remaining credit");
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(census(&p, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(census(&p, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    census(&p, &mut retry).expect("pristine retry");
    assert_eq!(retry, cost);
}
