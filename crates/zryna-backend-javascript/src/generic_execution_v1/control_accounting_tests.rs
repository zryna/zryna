//! Genuine control3 programs, public outcomes and explicitly seeded private source frontiers.
mod ordinary;
mod owner;
mod private_source;
#[path = "../../../zryna-driver/tests/generic_execution_v1_native_accounting_control/source.rs"]
mod source;
use sha2::{Digest, Sha256};
fn oracle() -> serde_json::Value {
    let bytes = include_bytes!("control_accounting_tests/RUNTIME_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "223d51b99323a0888dd7f81761cc68c7c0c456245b96098989a67d97f0f31f6b"
    );
    serde_json::from_slice(bytes).expect("pre-output portable source oracle")
}
fn row(case: &str) -> serde_json::Value {
    oracle()["cases"]
        .as_array()
        .expect("frozen rows")
        .iter()
        .find(|v| v["case"] == case)
        .expect("frozen genuine case")
        .clone()
}

mod private_effects;
