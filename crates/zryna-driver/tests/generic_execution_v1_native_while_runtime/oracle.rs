//! Fixed semantic expectations versus actual canonical pointers, UTF8, free bases, and mutation order.
use serde_json::Value;
use std::collections::BTreeMap;
#[path = "oracle/records.rs"]
mod records;
#[derive(Clone)]
pub(super) struct Fault {
    pub(super) kind: String,
    pub(super) ordinal: usize,
    expected: Value,
}
pub(super) fn row(case: &str) -> Value {
    let bytes = include_bytes!("RUNTIME_EXPECTED.json");
    assert_eq!(
        super::emit::digest(bytes),
        "e8a1761b0bd36f22dd78f90f00cf9ed777885f48b1026146fa97dbc22200060d"
    );
    let oracle: Value = serde_json::from_slice(bytes).expect("unchanged pre-output runtime oracle");
    oracle["cases"]
        .as_array()
        .expect("nine frozen positive cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("original case")
        .clone()
}
pub(super) fn runs(case: &str) -> Vec<Value> {
    row(case)["runs"].as_array().expect("fourteen frozen argument rows").clone()
}
pub(super) fn faults(run: &Value) -> Vec<Fault> {
    run["faults"]
        .as_array()
        .expect("pre-output fault rows")
        .iter()
        .map(|row| Fault {
            kind: text(row, "kind").into(),
            ordinal: number(&row["ordinal"]),
            expected: row.clone(),
        })
        .collect()
}
fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row[key].as_str().expect("actual trace string")
}
fn number(value: &Value) -> usize {
    usize::try_from(value.as_u64().expect("nonnegative trace count")).expect("small fixture count")
}
fn address(value: &str) -> u64 {
    u64::from_str_radix(value, 16).expect("actual pointer")
}
#[derive(Default)]
struct Actual {
    physical: BTreeMap<usize, (String, u64)>,
    live: BTreeMap<String, (usize, String, String)>,
    acquisitions: Vec<String>,
    events: Vec<String>,
    release_entries: Vec<usize>,
    released: Vec<usize>,
    freed: Vec<usize>,
    open_release: Option<usize>,
    failure: Vec<(String, usize)>,
    unchanged_sources: Vec<usize>,
    zeros: usize,
    clones: usize,
    concats: usize,
    done: Option<Value>,
}
pub(super) fn check(
    case: &str,
    run: &Value,
    fault: Option<&Fault>,
    trace: &[Value],
    metadata: &Value,
) {
    assert_eq!(
        metadata["canonical_source_sha256"],
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    check_entry(run, trace);
    let expected = run;
    let acquisitions =
        expected["acquisitions"].as_array().expect("frozen successful source/UTF8 sequence");
    let mut actual = Actual::default();
    for record in trace {
        actual.observe(case, record, acquisitions);
    }
    assert!(actual.live.is_empty() && actual.physical.is_empty() && actual.open_release.is_none());
    let releases =
        fault.map_or(
            &expected["pristine"]["release_acquisition_ids"],
            |f| &f.expected["release_acquisition_ids"],
        );
    let events = fault.map_or(&expected["pristine"]["events"], |f| &f.expected["events"]);
    assert_eq!(
        actual.events,
        events
            .as_array()
            .expect("frozen dynamic event interleaving")
            .iter()
            .map(|event| event.as_str().expect("source event").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        actual.released,
        releases.as_array().expect("fixed reverse cleanup").iter().map(number).collect::<Vec<_>>()
    );
    assert_eq!(actual.release_entries, actual.released);
    assert_eq!(actual.freed, actual.released);
    let count = acquisition_count(fault, acquisitions);
    assert_eq!(actual.acquisitions.len(), count);
    assert_eq!(actual.zeros, count + usize::from(fault.is_some()));
    assert_eq!(actual.failure, fault.map_or_else(Vec::new, |f| vec![(f.kind.clone(), f.ordinal)]));
    let unchanged = fault.map_or(&[][..], |f| {
        f.expected["unchanged_source_ids"].as_array().expect("fixed unchanged sources").as_slice()
    });
    assert_eq!(actual.unchanged_sources, unchanged.iter().map(number).collect::<Vec<_>>());
    if fault.is_none() {
        check_success(&actual, expected);
    } else {
        assert!(actual.done.is_none(), "controlled fault never returns success");
    }
}
fn check_success(actual: &Actual, expected: &Value) {
    let done = actual.done.as_ref().expect("ordinary return after all actual frees");
    assert_eq!(
        number(&done["allocations"]),
        number(&expected["pristine"]["language_string_allocations"])
    );
    assert_eq!(
        number(&done["attempts"]),
        number(&expected["pristine"]["language_string_allocations"])
    );
    assert_eq!(actual.clones, number(&expected["pristine"]["string_clone_calls"]));
    assert_eq!(number(&done["clones"]), actual.clones);
    assert_eq!(actual.concats, number(&expected["pristine"]["string_concat_calls"]));
    assert_eq!(number(&done["concats"]), actual.concats);
    assert_eq!(number(&done["zero_checks"]), actual.zeros);
}

fn acquisition_count(fault: Option<&Fault>, acquisitions: &[Value]) -> usize {
    fault.map_or(acquisitions.len(), |f| {
        if f.kind == "allocation" {
            return f.ordinal - 1;
        }
        let arity = match f.kind.as_str() {
            "stringClone" => 1,
            "stringConcat" => 2,
            other => panic!("unknown frozen fault {other}"),
        };
        acquisitions
            .iter()
            .enumerate()
            .filter(|(_, row)| row["source_ids"].as_array().expect("source roles").len() == arity)
            .nth(f.ordinal - 1)
            .expect("frozen source operation occurrence")
            .0
    })
}

fn check_entry(run: &Value, trace: &[Value]) {
    assert_eq!(trace.iter().filter(|row| text(row, "event") == "entry").count(), 1);
    let entry = trace.first().expect("actual typed scalar invocation");
    assert_eq!(text(entry, "event"), "entry");
    let expected = run["arguments"].as_array().expect("frozen scalar arguments");
    let actual = entry["arguments"].as_array().expect("actual native scalar arguments");
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        if let Some(boolean) = expected.as_bool() {
            assert_eq!(number(&entry["boolean"]), 1);
            assert_eq!(actual.as_i64(), Some(i64::from(boolean)));
        } else {
            assert_eq!(number(&entry["boolean"]), 0);
            assert_eq!(actual.as_i64(), expected.as_i64());
        }
    }
}
