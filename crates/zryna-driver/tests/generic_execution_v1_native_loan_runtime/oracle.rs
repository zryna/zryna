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
        "65d001167f278c04ac4c9997b2a6319cfb7e0aefbd23834fe75ce23be909ff3d"
    );
    let oracle: Value = serde_json::from_slice(bytes).expect("unchanged pre-output runtime oracle");
    oracle["cases"]
        .as_array()
        .expect("three frozen cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("original case")
        .clone()
}
pub(super) fn faults(case: &str) -> Vec<Fault> {
    row(case)["faults"]
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
    release_entries: Vec<usize>,
    released: Vec<usize>,
    freed: Vec<usize>,
    open_release: Option<usize>,
    failure: Vec<(String, usize)>,
    old_entries: Vec<usize>,
    old_complete: Vec<usize>,
    committed: Vec<usize>,
    unchanged_sources: Vec<usize>,
    unchanged_roots: Vec<usize>,
    root_address: Option<String>,
    zeros: usize,
    clones: usize,
    done: Option<Value>,
}
pub(super) fn check(case: &str, fault: Option<&Fault>, trace: &[Value], metadata: &Value) {
    assert_eq!(
        metadata["canonical_source_sha256"],
        "94ce0a5f0d40458a7dcc234b93f98a2d18c985ff1f60e285c238516bea3d0e26"
    );
    let bytes = include_bytes!("OBSERVER_EXPECTED.json");
    assert_eq!(
        super::emit::digest(bytes),
        "da078a27bdb5c438d3dac35ac5f2a340369519cbda4672fbe53328646896d58a"
    );
    let observed: Value = serde_json::from_slice(bytes).expect("pre-output UTF8/pointer oracle");
    let acquisitions =
        observed["cases"][case]["acquisitions"].as_array().expect("frozen successful sequence");
    let mut actual = Actual::default();
    for record in trace {
        actual.observe(record, acquisitions);
    }
    assert!(actual.live.is_empty() && actual.physical.is_empty() && actual.open_release.is_none());
    let expected = row(case);
    let releases =
        fault.map_or(
            &expected["pristine"]["release_acquisition_ids"],
            |f| &f.expected["release_acquisition_ids"],
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
    check_replacements(case, count, &actual);
    check_unchanged(case, fault, &actual);
    if fault.is_none() {
        let done = actual.done.expect("ordinary return after all actual frees");
        assert_eq!(
            number(&done["allocations"]),
            number(&expected["pristine"]["language_string_allocations"])
        );
        assert_eq!(actual.clones, number(&expected["pristine"]["string_clone_calls"]));
        assert_eq!(number(&done["clones"]), actual.clones);
        assert_eq!(number(&done["zero_checks"]), actual.zeros);
    } else {
        assert!(actual.done.is_none(), "controlled fault never returns success");
    }
}
fn acquisition_count(fault: Option<&Fault>, acquisitions: &[Value]) -> usize {
    fault.map_or(acquisitions.len(), |f| {
        if f.kind == "allocation" {
            f.ordinal - 1
        } else {
            acquisitions
                .iter()
                .enumerate()
                .filter(|(_, row)| number(&row["source_id"]) != 0)
                .nth(f.ordinal - 1)
                .expect("frozen clone occurrence")
                .0
        }
    })
}
fn check_replacements(case: &str, count: usize, actual: &Actual) {
    if case == "generic-replace" {
        let replaced = count.saturating_sub(1);
        assert_eq!(actual.old_entries, (1..=replaced).collect::<Vec<_>>());
        assert_eq!(actual.old_complete, actual.old_entries);
        assert_eq!(
            actual.committed,
            if count == 3 {
                vec![2, 3]
            } else if count == 2 {
                vec![2]
            } else {
                vec![]
            }
        );
    } else {
        assert!(actual.old_entries.is_empty() && actual.committed.is_empty());
    }
}
fn check_unchanged(case: &str, fault: Option<&Fault>, actual: &Actual) {
    let expected = fault.and_then(|f| match (case, f.kind.as_str(), f.ordinal) {
        ("generic-replace", "allocation", 2)
        | ("generic-replace", "stringClone", 1)
        | ("fallible-cleanup", "allocation", 4)
        | ("fallible-cleanup", "stringClone", 2) => Some(1),
        ("fallible-cleanup", "allocation", 3) | ("fallible-cleanup", "stringClone", 1) => Some(2),
        _ => None,
    });
    assert_eq!(actual.unchanged_sources, expected.into_iter().collect::<Vec<_>>());
    assert_eq!(
        actual.unchanged_roots,
        if case == "generic-replace"
            && fault.is_some_and(|f| f.kind == "allocation" && f.ordinal == 3)
        {
            vec![2]
        } else {
            vec![]
        }
    );
}
