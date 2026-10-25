//! Independently source-derived call/frame addendum; no oracle regeneration from target output.
use serde_json::Value;
use sha2::{Digest, Sha256};
pub(super) fn load() -> Value {
    let bytes = include_bytes!(
        "../../../../../tests/m7-generic-native-mixed-runtime/EXPECTED_ADDENDUM.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "ef0a2f7bee9577d1ab0d791051620b776b70e689087f935a03b11b3a9157203e"
    );
    serde_json::from_slice(bytes).expect("pre-output native source call expectations")
}
pub(super) fn calls<'a>(addendum: &'a Value, fault: Option<&Value>) -> &'a Value {
    fault.map_or(&addendum["pristine_calls"], |fault| {
        &addendum["fault_calls"]
            .as_array()
            .expect("35 frozen rows")
            .iter()
            .find(|row| row["kind"] == fault["kind"] && row["ordinal"] == fault["ordinal"])
            .expect("exact existing fault")["calls"]
    })
}
