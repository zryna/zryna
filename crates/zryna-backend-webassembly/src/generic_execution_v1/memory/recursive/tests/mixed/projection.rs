//! Complete existing observer String projection, independent of pointer values chosen by runtime.
use super::session::Session;
use serde_json::Value;
use std::collections::BTreeMap;
type Live = BTreeMap<u32, (u32, Vec<u32>)>;
fn number(value: &Value) -> u32 {
    u32::try_from(value.as_u64().expect("frozen unsigned word")).expect("u32 oracle word")
}
fn bytes(value: &Value) -> Vec<u32> {
    value.as_array().expect("frozen byte/id array").iter().map(number).collect()
}
fn source(row: &Value, live: &Live) -> u32 {
    let id = row.get("source_id").map_or(0, number);
    if id == 0 { 0 } else { live.get(&id).expect("original source stays live").0 }
}
fn failure_record(fault: Option<&Value>) -> Option<&Value> {
    fault.and_then(|row| row.get("failure_String_record")).filter(|record| !record.is_null())
}
fn words(session: &mut Session) -> Vec<u32> {
    let count = session.get(0x4000_0000);
    assert!((0..=4096).contains(&count), "complete bounded String projection");
    (0..u32::try_from(count).expect("bounded nonnegative count"))
        .map(|index| session.get(0x4000_0001 + index).cast_unsigned())
        .collect()
}
fn acquire(record: [u32; 4], payload: &[u32], oracle: &Value, live: &mut Live) {
    let [kind, id, pointer, original] = record;
    let row = &oracle["pristine_allocations"][usize::try_from(
        id.checked_sub(1).expect("positive acquisition identity"),
    )
    .expect("bounded id")];
    assert_eq!(row["kind"], "String");
    assert_eq!(payload, bytes(&row["utf8"]));
    assert_eq!(kind, if row["role"] == "StringClone" { 2 } else { 1 });
    assert_eq!(original, source(row, live));
    assert!(
        pointer > 0 && live.values().all(|value| value.0 != pointer),
        "successful owner has a fresh live physical pointer"
    );
    assert!(live.insert(id, (pointer, payload.to_vec())).is_none(), "new acquisition identity");
}
pub(super) fn check(
    session: &mut Session,
    oracle: &Value,
    expected: &Value,
    fault: Option<&Value>,
) {
    let status = session.get(0);
    let words = words(session);
    // Print exact raw observed words before semantic assertions, including on a failing test.
    eprintln!(
        "portable-mixed-String-observation {}",
        serde_json::json!({
        "fault":fault.map(|row| (&row["kind"], &row["ordinal"])),
        "status":status,"words":&words,
        "limits":"String projection/status/gate only; no full Vec/control release-order claim"})
    );
    assert_eq!(status, fault.map_or(0, |row| number(&row["wasm_status_getter_0"])).cast_signed());
    let mut live = Live::new();
    let mut acquired = Vec::new();
    let mut released = Vec::new();
    let mut failures = 0;
    let mut at = 0;
    while at < words.len() {
        assert!(words.len() - at >= 5, "complete record header");
        let [kind, id, pointer, original, length]: [u32; 5] =
            words[at..at + 5].try_into().expect("five header words");
        at += 5;
        let length = usize::try_from(length).expect("bounded length");
        assert!(length <= words.len() - at, "complete payload, no ignored suffix");
        let payload = &words[at..at + length];
        at += length;
        assert!(payload.iter().all(|byte| *byte <= 255), "actual UTF8 bytes");
        match kind {
            1 | 2 => {
                acquire([kind, id, pointer, original], payload, oracle, &mut live);
                acquired.push(id);
            }
            3 => {
                assert_eq!(original, 0);
                assert_eq!(
                    live.remove(&id).expect("release original live owner"),
                    (pointer, payload.to_vec())
                );
                released.push(id);
            }
            _ => {
                let row =
                    failure_record(fault).expect("Vec/control failures invent no String record");
                assert_eq!(
                    [kind, id, pointer],
                    [number(&row["kind"]), number(&row["id"]), number(&row["pointer"])]
                );
                assert_eq!(original, source(row, &live));
                assert_eq!(payload, bytes(&row["utf8"]));
                failures += 1;
            }
        }
    }
    assert_eq!(acquired, bytes(&expected["String_projection"]["acquired_ids"]));
    assert_eq!(released, bytes(&expected["String_projection"]["released_ids"]));
    assert_eq!(failures, usize::from(failure_record(fault).is_some()));
    assert!(live.is_empty(), "all String owners released exactly once");
}
