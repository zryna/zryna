//! Existing qualified source worlds, actual aggregate glue and immutable counter projection.
use super::{Context, runtime::accounting};
use sha2::{Digest, Sha256};
#[path = "../../../zryna-driver/tests/generic_execution_v1_loan_source_seal/source.rs"]
mod catalogue;
mod owner;
#[path = "../../../zryna-driver/tests/generic_execution_v1_partial_static_source_seal/source.rs"]
mod partial;
mod prefix;
mod private_source;
mod source;
fn oracle() -> serde_json::Value {
    let bytes = include_bytes!("aggregate_accounting_tests/HELD_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "868cbef911dfa4bfa463b081339738186c103dfe030e3409b93b8fefc0019a11"
    );
    serde_json::from_slice(bytes).expect("source/law-derived immutable numeric projection")
}
