//! Existing source formulas versus actual executed reserved wire and complete canonical rows.
use serde_json::Value;
use zryna_ir::generic_v1::execution_v1::invocation::{self, Outcome, SourceStatus};
fn numbers(value: &Value) -> Vec<u64> {
    value.as_array().expect("number list").iter().map(|n| n.as_u64().expect("number")).collect()
}
pub(super) fn check(row: &Value, retry: bool, stdout: &[u8], bytes: &[u8]) {
    transport(row, retry, stdout);
    let actual = super::ledger::Ledger::parse(bytes);
    assert_eq!(
        actual.released,
        numbers(&row[if retry { "retry_releases" } else { "fault_releases" }])
    );
    let expected = row["retry_acquisitions"].as_array().expect("frozen payloads");
    let count = if retry {
        expected.len()
    } else {
        usize::try_from(row["fault_allocations"].as_u64().expect("prefix count"))
            .expect("small count")
    };
    assert_eq!(actual.acquired.len(), count);
    for (actual, expected) in actual.acquired.iter().zip(expected) {
        payload(actual, expected);
    }
    roles(row, retry, &actual.calls, &actual.failures);
}
fn transport(row: &Value, retry: bool, stdout: &[u8]) {
    let outcome = invocation::decode(stdout, zryna_abi::ScalarType::I32)
        .expect("actual exact20LE reserved transport");
    if retry {
        let Outcome::Value(zryna_abi::ScalarValue::I32(value)) = outcome else {
            panic!("pristine typed scalar")
        };
        assert_eq!(i64::from(value), row["retry_result"]);
    } else {
        let Outcome::Trap(trap) = outcome else { panic!("actual source-owned trap") };
        assert_eq!(u64::from(SourceStatus::Trap(trap).code()), row["source_code"]);
        assert_eq!(&stdout[12..16], &[0, 0, 0, 0], "failure publishes no scalar");
    }
}
fn payload(actual: &[u64; 8], expected: &Value) {
    assert_eq!(actual[2], expected["kind"]);
    assert_eq!(actual[5], expected["source"]);
    let hex = expected["utf8_hex"].as_str().expect("payload");
    assert_eq!(actual[6], (hex.len() / 2) as u64);
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect::<Vec<_>>();
    assert_eq!(&actual[7].to_le_bytes()[..bytes.len()], bytes);
}
fn roles(row: &Value, retry: bool, calls: &[(u64, u64)], failures: &[(u64, u64, u64)]) {
    let pins = super::source::pins();
    let expected = &pins["roles"][row["family"].as_str().expect("family")]
        [if retry { "pristine" } else { "fault" }];
    assert_eq!(calls.iter().map(|(role, _)| *role).collect::<Vec<_>>(), numbers(expected));
    let injected = !retry && !row["canonical_status"].is_null();
    let expected = if injected {
        vec![(
            row["fault_role"].as_u64().expect("role"),
            row["fault_ordinal"].as_u64().expect("ordinal"),
            row["canonical_status"].as_u64().expect("status"),
        )]
    } else {
        vec![]
    };
    assert_eq!(failures, expected);
    let nonzero = calls.iter().copied().filter(|(_, status)| *status != 0).collect::<Vec<_>>();
    let expected = if injected { vec![(expected[0].0, expected[0].2)] } else { vec![] };
    assert_eq!(nonzero, expected, "only the designated canonical call can fail");
}
