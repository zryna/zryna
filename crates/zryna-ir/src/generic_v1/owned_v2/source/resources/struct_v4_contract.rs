//! PROPOSED RED private resource tests. Register in resources.rs under cfg(test).
//! `Budget::owned_operation` is a proposed private charging hook used before retaining extensions.
use super::super::Owned;
use super::*;

#[test]
fn plain_and_generic_struct_key_copies_use_the_existing_nominal_work_credit() {
    let limit = super::super::super::plan::MAX_OWNER_UNITS;
    for key in [
        vec![0x10, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0x12, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
    ] {
        let bytes = key.len();
        let ty = Closed::Stored(key);
        let mut budget = Budget::original(limit % bytes);
        for _ in 0..limit / bytes {
            budget.nominal_key(&ty, 1).expect("all Struct key copies charged");
        }
        assert_eq!(budget.nominal_used(), limit);
        assert!(budget.nominal_key(&ty, 1).is_err());
        assert_eq!(budget.nominal_used(), limit, "first excess is atomic");
        assert!(budget.nominal_key(&ty, usize::MAX).is_err());
        assert_eq!(budget.nominal_used(), limit, "overflow cannot release/reset work credit");
    }
}

#[test]
fn declaration_order_field_operands_use_one_program_counter_without_function_reset() {
    let operation = Owned::StructConstruct { ty: 0, fields: vec![0; 1024] };
    let mut budget = Budget::default();
    // 16 source functions * 16 constructors/function * 1024 fields = 262,144.
    // Copy IDs intentionally repeat: these tests isolate metering from later typed/owner proof.
    for _ in 0..16 {
        let mut next_function = budget;
        for _ in 0..16 {
            next_function
                .owned_operation(&operation)
                .expect("exact independently counted aggregate operands");
        }
        budget = next_function;
    }
    assert_eq!(budget.operands, limits::MAX_AGGREGATE_OPERANDS);
    assert!(budget.owned_operation(&Owned::StructConstruct { ty: 0, fields: vec![0] }).is_err());
    assert_eq!(budget.operands, limits::MAX_AGGREGATE_OPERANDS, "first extra rejects atomically");
    // Observation publishes no aggregate and cannot reset the exhausted constructor counter.
    budget
        .owned_operation(&Owned::CopyStructField { value: 0, ordinal: 0 })
        .expect("no stored constructor operands");
    assert!(budget.owned_operation(&Owned::StructConstruct { ty: 0, fields: vec![0] }).is_err());
}
