//! Existing container JSON String projection compared completely to unchanged source oracle.
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
fn number(row: &Value) -> u64 {
    row.as_u64().expect("frozen integer")
}
fn pointer(row: &Value) -> u64 {
    u64::from_str_radix(row.as_str().expect("actual hex pointer"), 16).expect("pointer")
}
fn bytes(row: &Value) -> String {
    let mut output = String::new();
    for byte in row.as_array().expect("source UTF8") {
        write!(&mut output, "{:02x}", number(byte)).expect("write source UTF8 hex");
    }
    output
}
fn source(row: &Value, live: &BTreeMap<u64, (u64, String)>) -> u64 {
    let id = row.get("source_id").map_or(0, number);
    if id == 0 { 0 } else { live.get(&id).expect("original source stays live").0 }
}
pub(super) fn check(records: &[Value], oracle: &Value, expected: &Value, fault: Option<&Value>) {
    counts(records, expected);
    let mut live = BTreeMap::new();
    let mut acquired = Vec::new();
    let mut released = Vec::new();
    let mut failures = 0;
    for row in records {
        if row["event"] == "count" {
            assert_eq!(row["status"], 0);
            continue;
        }
        assert!(matches!(row["event"].as_str(), Some("acquire" | "release" | "failure")));
        if row["event"] != "failure" {
            assert_eq!(row["kind"], "String");
        }
        let id = number(&row["id"]);
        let actual = pointer(&row["pointer"]);
        let original = pointer(&row["source"]);
        let payload = row["bytes_hex"].as_str().expect("actual complete bytes");
        if row["event"] == "acquire" {
            let own = &oracle["pristine_allocations"][usize::try_from(id - 1).expect("identity")];
            assert_eq!(own["kind"], "String");
            assert_eq!(payload, bytes(&own["utf8"]));
            assert_eq!(number(&row["length"]), number(&own["length"]));
            assert_eq!(original, source(own, &live));
            assert_ne!(actual, 0);
            assert!(live.values().all(|(pointer, _)| *pointer != actual));
            assert!(live.insert(id, (actual, payload.to_owned())).is_none());
            acquired.push(id);
        } else if row["event"] == "release" {
            assert_eq!(original, 0);
            assert_eq!(live.remove(&id), Some((actual, payload.to_owned())));
            assert_eq!(number(&row["length"]) * 2, u64::try_from(payload.len()).expect("length"));
            released.push(id);
        } else {
            let failure = fault
                .and_then(|row| row.get("failure_String_record"))
                .filter(|row| !row.is_null())
                .expect("Vec/control failures invent no String failure record");
            assert_eq!((id, actual), (number(&failure["id"]), number(&failure["pointer"])));
            assert_eq!(
                row["kind"],
                if failure["kind"] == 5 { "string-clone" } else { "allocation" }
            );
            assert_eq!(original, source(failure, &live));
            assert_eq!(payload, bytes(&failure["utf8"]));
            assert_eq!(number(&row["length"]) * 2, u64::try_from(payload.len()).expect("length"));
            failures += 1;
        }
    }
    let ids = |key: &str| {
        expected["String_projection"][key]
            .as_array()
            .expect("source projection")
            .iter()
            .map(number)
            .collect::<Vec<_>>()
    };
    assert_eq!(acquired, ids("acquired_ids"));
    assert_eq!(released, ids("released_ids"));
    assert!(live.is_empty());
    assert_eq!(
        failures,
        usize::from(
            fault
                .and_then(|row| row.get("failure_String_record"))
                .is_some_and(|row| !row.is_null())
        )
    );
}

fn counts(records: &[Value], expected: &Value) {
    let rows = records.iter().filter(|row| row["event"] == "count").collect::<Vec<_>>();
    let acquired = expected
        .get("acquired_ids")
        .unwrap_or(&expected["acquisition_ids"])
        .as_array()
        .expect("source physical identities");
    let count = if acquired.iter().any(|id| id.as_u64() == Some(24)) {
        10
    } else if acquired.iter().any(|id| id.as_u64() == Some(22)) {
        4
    } else {
        0
    };
    let values = [
        ("strongClone", 1, 1, 2, 1),
        ("strongReleaseBegin", 2, 1, 1, 1),
        ("strongReleaseBegin", 1, 1, 0, 1),
        ("strongReleaseFinish", 0, 1, 0, 0),
        ("weakDowngrade", 1, 1, 1, 2),
        ("weakClone", 1, 2, 1, 3),
        ("weakRelease", 1, 3, 1, 2),
        ("weakRelease", 1, 2, 1, 1),
        ("strongReleaseBegin", 1, 1, 0, 1),
        ("strongReleaseFinish", 0, 1, 0, 0),
    ];
    assert_eq!(rows.len(), count);
    for (index, row) in rows.iter().enumerate() {
        let (name, strong, weak, after_s, after_w) = values[index];
        assert_eq!(
            (
                row["name"].as_str().expect("actual count role"),
                number(&row["beforeStrong"]),
                number(&row["beforeWeak"]),
                number(&row["afterStrong"]),
                number(&row["afterWeak"])
            ),
            (name, strong, weak, after_s, after_w)
        );
        let base = if index < 4 { 0 } else { 4 };
        assert_ne!(pointer(&row["pointer"]), 0);
        assert_eq!(row["pointer"], rows[base]["pointer"], "same actual control per lexical scope");
    }
}
