//! Independent unchanged source trace expectations plus actual native allocation correspondence.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
pub(super) fn case(name: &str) -> Value {
    let bytes = include_bytes!("OBSERVED_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "e4ffbd6b69f42d8d023fa9265b51a3bc1b08cebdc679caaa0cd3bb852032bad3"
    );
    let oracle: Value =
        serde_json::from_slice(bytes).expect("frozen pre-output language-level oracle");
    oracle["cases"]
        .as_array()
        .expect("twelve cases")
        .iter()
        .find(|row| row["case"] == name)
        .expect("unchanged source case")
        .clone()
}
fn number(row: &Value, key: &str) -> u64 {
    row[key].as_u64().expect("actual unsigned observation")
}
fn pointer(row: &Value, key: &str) -> u64 {
    u64::from_str_radix(row[key].as_str().expect("actual pointer hex"), 16)
        .expect("native64 pointer")
}
fn events<'a>(records: &'a [Value], name: &'a str) -> impl Iterator<Item = &'a Value> {
    records.iter().filter(move |row| row["event"] == name)
}
pub(super) fn check(expected: &Value, records: &[Value], metadata: &Value) -> Value {
    let canonical = events(records, "canonical")
        .map(|row| {
            let mut event = row.as_object().expect("actual event object").clone();
            event.remove("event");
            Value::Object(event)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        json!(canonical),
        expected["trace"],
        "exact canonical entry order/status and source identity"
    );
    let acquired = events(records, "acquired").collect::<Vec<_>>();
    let projected = acquired.iter().map(|row| {
        if row["kind"] == "String" {
            json!({"owner":row["owner"],"kind":row["kind"],"utf8":row["utf8"],"source":row["source"]})
        } else { json!({"owner":row["owner"],"kind":row["kind"],"capacity":row["capacity"]}) }
    }).collect::<Vec<_>>();
    assert_eq!(
        json!(projected),
        expected["acquired"],
        "actual acquisitions, independent of helper IDs"
    );
    let released = events(records, "released").map(|row| number(row, "owner")).collect::<Vec<_>>();
    assert_eq!(
        json!(released),
        expected["released"],
        "exact source owner reverse cleanup including Reserve retirement"
    );
    let failed_outputs = events(records, "failed-output").collect::<Vec<_>>();
    assert_eq!(failed_outputs.len(), usize::from(expected.get("fault").is_some()));
    assert!(
        failed_outputs
            .iter()
            .all(|row| row["canonical_zero"] == true && row["poisoned_after_return"] == true)
    );
    attempts(expected, records, &acquired);
    allocations(records, &acquired, metadata);
    json!({"canonical":canonical,"acquired":acquired,"released":released,
        "physical_attempts":expected["physicalAttempts"],"all_acquired_released_once":true})
}
fn attempts(expected: &Value, records: &[Value], acquired: &[&Value]) {
    let attempts = events(records, "attempt").collect::<Vec<_>>();
    assert_eq!(
        u64::try_from(attempts.len()).expect("finite attempts"),
        number(expected, "physicalAttempts")
    );
    for (index, attempt) in attempts.iter().enumerate() {
        assert_eq!(number(attempt, "ordinal"), u64::try_from(index + 1).expect("finite ordinal"));
        assert!(number(attempt, "bytes") > 0);
        assert_eq!(attempt["failed"], expected["fault"]["ordinal"] == attempt["ordinal"]);
    }
    assert_eq!(attempts.iter().filter(|row| row["failed"] == false).count(), acquired.len());
}
fn allocations(records: &[Value], acquired: &[&Value], metadata: &Value) {
    for (index, owner) in acquired.iter().enumerate() {
        assert_eq!(number(owner, "owner"), u64::try_from(index + 1).expect("finite identity"));
        let actual = events(records, "attempt")
            .find(|row| row["base"] == owner["base"])
            .expect("actual successful malloc base");
        assert_eq!(actual["failed"], false);
        assert_eq!(actual["bytes"], owner["allocationBytes"]);
        let offset = pointer(owner, "pointer")
            .checked_sub(pointer(owner, "base"))
            .expect("payload above base");
        assert!(
            offset
                .checked_add(number(owner, "payloadBytes"))
                .is_some_and(|end| end <= number(owner, "allocationBytes"))
        );
        let frees = events(records, "released")
            .filter(|row| row["owner"] == owner["owner"])
            .collect::<Vec<_>>();
        assert_eq!(
            frees.len(),
            1,
            "actual allocation retired exactly once even on controlled trap"
        );
        assert_eq!(frees[0]["base"], owner["base"]);
        assert_eq!(frees[0]["pointer"], owner["pointer"]);
        assert_eq!(frees[0]["utf8"], owner["utf8"], "String unchanged through actual release");
        if owner["kind"] == "Vec buffer" {
            vector(records, owner, metadata);
        } else {
            assert_eq!(number(owner, "payloadBytes"), number(owner, "capacity"));
        }
    }
    assert_eq!(events(records, "released").count(), acquired.len());
}
fn vector(records: &[Value], owner: &Value, metadata: &Value) {
    let actual = events(records, "vec-metadata")
        .find(|row| row["owner"] == owner["owner"])
        .expect("actual helper element and extent");
    let retained = metadata["element_layouts"]
        .as_array()
        .expect("same NEWseal metadata")
        .iter()
        .find(|row| row["element"] == actual["element"])
        .expect("real retained element identity");
    assert_eq!(actual["stride"], retained["stride"]);
    assert_eq!(actual["alignment"], retained["alignment"]);
    assert_eq!(actual["capacity"], owner["capacity"]);
    assert_eq!(
        number(owner, "payloadBytes"),
        number(owner, "capacity") * number(retained, "stride")
    );
    assert_eq!(pointer(owner, "pointer") % number(retained, "alignment"), 0);
}
