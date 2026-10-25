//! Additive transport expectations stay attached to unchanged prior cleanup oracles.
use std::path::Path;

fn original(freeze: &serde_json::Value, path: &str) -> serde_json::Value {
    let row = &freeze["original_inputs"][path];
    let bytes = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path))
        .expect("original oracle");
    assert_eq!(super::emit::digest(&bytes), row["sha256"]);
    assert_eq!(u64::try_from(bytes.len()).expect("bytes"), row["bytes"]);
    serde_json::from_slice(&bytes).expect("unchanged old oracle JSON")
}
pub(super) fn check(freeze: &serde_json::Value) {
    let concat = original(
        freeze,
        "crates/zryna-driver/tests/generic_execution_v1_native_concat_runtime/RUNTIME_EXPECTED.json",
    );
    let control = original(
        freeze,
        "crates/zryna-driver/tests/generic_execution_v1_native_containers/RUNTIME_EXPECTED.json",
    );
    let index = original(
        freeze,
        "crates/zryna-driver/tests/generic_execution_v1_native_indexed_runtime/OBSERVED_EXPECTED_FREEZE.json",
    );
    let find = |value: &serde_json::Value, name: &str| {
        value["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|row| row["case"] == name)
            .expect("original row")
            .clone()
    };
    let concat = find(&concat, "nested-temporaries");
    let control = find(&control, "shared-weak-count");
    let index = find(&index, "vec-shared-clone");
    for row in freeze["source_map"].as_array().expect("five selected source edges") {
        match row["family"].as_str().expect("family") {
            "concat" => {
                let fault = concat["faults"]
                    .as_array()
                    .expect("faults")
                    .iter()
                    .find(|r| r["kind"] == "allocation" && r["ordinal"] == 3)
                    .expect("same live prefix");
                assert_eq!(row["fault_releases"], fault["release_acquisition_ids"]);
                assert_eq!(row["retry_releases"], concat["pristine"]["release_acquisition_ids"]);
                assert_eq!(row["retry_result"], concat["pristine"]["scalar_result"]);
                assert_eq!(
                    row["retry_acquisitions"].as_array().expect("selected payloads").len(),
                    concat["acquisitions"].as_array().expect("old payloads").len()
                );
                for (a, b) in row["retry_acquisitions"]
                    .as_array()
                    .expect("selected payloads")
                    .iter()
                    .zip(concat["acquisitions"].as_array().expect("original payloads"))
                {
                    assert_eq!(a["utf8_hex"], b["utf8_hex"]);
                }
            }
            "control" => {
                let fault = control["count_faults"]
                    .as_array()
                    .expect("faults")
                    .iter()
                    .find(|r| r["kind"] == "weak-clone-status")
                    .expect("same count edge");
                assert_eq!(row["fault_releases"], fault["releases"]);
                assert_eq!(row["retry_releases"], control["releases"]);
                assert_eq!(row["canonical_status"], fault["injected_declared_status"]);
                assert_eq!(row["retry_result"], control["observation"]);
                assert_eq!(
                    row["retry_acquisitions"][0]["utf8_hex"],
                    control["string_acquisition_utf8_hex"]["1"]
                );
            }
            "index" => {
                for (key, args) in [("fault_releases", -1), ("retry_releases", 0)] {
                    let original = index["normal"]
                        .as_array()
                        .expect("normal")
                        .iter()
                        .find(|r| r["arguments"] == serde_json::json!([args]))
                        .expect("same baked argument");
                    assert_eq!(row[key], original["released"]);
                }
            }
            _ => panic!("fixed existing source catalogue"),
        }
    }
}
