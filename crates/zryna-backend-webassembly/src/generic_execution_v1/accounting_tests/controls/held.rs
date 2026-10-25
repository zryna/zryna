//! Nineteen representable fixed-debt recipes; synthetic seeds do not confer source authority.
use super::{harness::Harness, with_model};
use serde_json::Value;
mod runner;
#[test]
fn real_control_adapters_match_nineteen_representable_fixed_debt_rows() {
    let oracle: Value = serde_json::from_str(include_str!("CONTROL_HELD_EXPECTED_FREEZE.json"))
        .expect("immutable twenty-row law");
    for provider in super::source::PROVIDERS {
        with_model("control-live-count-call-loop", provider, |m, wire| {
            let harness = Harness::new(m, wire, &format!("held-{provider}"));
            let mut reached = 0;
            for row in oracle["cases"].as_array().expect("fixed rows") {
                if row["id"] == "prospective-debt-add-overflow-is-atomic-denial" {
                    continue;
                }
                runner::run(&harness, row);
                reached += 1;
            }
            assert_eq!(reached, 19, "only actual representable recipes are target observations");
        });
    }
}
#[test]
fn generic_u64_debt_overflow_remains_model_only_without_truncation() {
    let oracle: Value = serde_json::from_str(include_str!("CONTROL_HELD_EXPECTED_FREEZE.json"))
        .expect("immutable model rows");
    let row = oracle["cases"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["id"] == "prospective-debt-add-overflow-is-atomic-denial")
        .expect("retained generic overflow row");
    let debt = row["events"][0]["request"]["new_hold"].as_u64().expect("u64 request");
    assert_eq!(debt, u64::MAX);
    assert!(u32::try_from(debt).is_err(), "no representable fixed-debt target effect exists");
    assert_eq!(row["events"][0]["fault"], "Denied");
    // The independent ABI model owns this arbitrary-request arithmetic case; no Wasm call is claimed.
}
