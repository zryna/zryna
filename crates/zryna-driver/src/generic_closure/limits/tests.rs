use super::*;
#[test]
fn exact_physical_call_and_final_reserve_boundaries_are_atomic() {
    let mut ledger = Ledger::default();
    ledger.source(80).expect("actual small source byte count");
    ledger.calls = MAX_MODULE_PROVIDER_CALLS - 2;
    ledger.provider(80, 1, false).expect("last discovery call plus final reserve");
    assert_eq!(ledger.calls, MAX_MODULE_PROVIDER_CALLS - 1);
    let before = (ledger.calls, ledger.provider_bytes);
    assert!(ledger.provider(80, 1, false).is_err());
    assert_eq!((ledger.calls, ledger.provider_bytes), before);
    ledger.provider(80, 1, true).expect("exact final complete-map call");
    assert_eq!(ledger.calls, MAX_MODULE_PROVIDER_CALLS);
    let before = (ledger.calls, ledger.provider_bytes);
    assert!(ledger.provider(80, 1, true).is_err());
    assert_eq!((ledger.calls, ledger.provider_bytes), before);
}
#[test]
fn source_and_provider_bytes_accept_exact_reject_first_extra_and_prevent_overflow() {
    let mut ledger = Ledger::default();
    ledger.source(MAX_MODULE_SOURCE_BYTES).expect("exact inherited source ceiling");
    assert!(ledger.source(1).is_err());
    assert_eq!(ledger.bytes, MAX_MODULE_SOURCE_BYTES);
    ledger
        .provider(MAX_MODULE_SOURCE_BYTES, 1, false)
        .expect("first full map plus exact final reserve");
    ledger.provider(MAX_MODULE_SOURCE_BYTES, 1, true).expect("exact cumulative provider ceiling");
    assert_eq!(ledger.provider_bytes, MAX_MODULE_PROVIDER_SOURCE_BYTES);
    let before = (ledger.calls, ledger.provider_bytes);
    assert!(ledger.provider(1, 1, true).is_err());
    assert_eq!((ledger.calls, ledger.provider_bytes), before);
    assert!(ledger.provider(MAX_MODULE_SOURCE_BYTES, usize::MAX, false).is_err());
    assert_eq!((ledger.calls, ledger.provider_bytes), before);
}
#[test]
fn comparison_consumes_two_physical_calls_and_two_final_reserve_slots() {
    let mut ledger = Ledger::default();
    ledger.source(80).expect("small actual source bytes");
    ledger.calls = MAX_MODULE_PROVIDER_CALLS - 4;
    ledger.provider(80, 2, false).expect("two physical analyses plus two reserved");
    assert_eq!(ledger.calls, MAX_MODULE_PROVIDER_CALLS - 2);
    ledger.provider(80, 2, true).expect("two final physical analyses");
    assert_eq!(ledger.calls, MAX_MODULE_PROVIDER_CALLS);
}
