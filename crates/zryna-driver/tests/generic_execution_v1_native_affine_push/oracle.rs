//! Actual canonical allocation/free bases, payload identities and count transitions versus fixed requirements.
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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
        "d465a84c2aec0d750f3f0a1765b66344fa859137605ebb41ba364b27a72460b4"
    );
    let expected: Value =
        serde_json::from_slice(bytes).expect("frozen affine prepared-RHS native oracle");
    assert_eq!(expected["case"], case);
    expected
}
pub(super) fn faults(case: &str) -> Vec<Fault> {
    row(case)["faults"]
        .as_array()
        .expect("pre-execution fault rows")
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
    usize::try_from(value.as_u64().expect("nonnegative bounded count"))
        .expect("small fixture count")
}
fn address(value: &str) -> u64 {
    u64::from_str_radix(value, 16).expect("actual machine pointer")
}
#[derive(Default)]
struct Actual {
    physical: HashMap<String, (usize, u64)>,
    freed: HashSet<usize>,
    bindings: HashMap<usize, String>,
    all: HashMap<String, (usize, String)>,
    live: HashMap<String, (usize, String)>,
    released: Vec<usize>,
    kinds: Vec<String>,
    metadata: HashSet<String>,
    reserve: Option<String>,
    failures: usize,
    mallocs: usize,
}
pub(super) fn check(case: &str, fault: Option<&Fault>, records: &[Value], metadata: &Value) {
    let expected = row(case);
    let mut actual = Actual::default();
    for record in records {
        actual.observe(fault, record, metadata, &expected);
    }
    assert!(actual.live.is_empty() && actual.physical.is_empty() && actual.metadata.is_empty());
    assert_eq!(actual.freed.len(), actual.mallocs);
    assert_eq!(actual.kinds.len(), actual.mallocs);
    assert!(actual.bindings.is_empty());
    let releases = fault.map_or(&expected["releases"], |fault| &fault.expected["releases"]);
    assert_eq!(
        actual.released,
        releases.as_array().expect("frozen releases").iter().map(number).collect::<Vec<_>>()
    );
    let acquisitions = fault.map_or_else(
        || expected["acquisitions"].as_array().expect("success allocations").len(),
        |fault| number(&fault.expected["successful_acquisitions"]),
    );
    assert_eq!(actual.kinds.len(), acquisitions);
    assert_eq!(
        actual.kinds.iter().map(String::as_str).collect::<Vec<_>>(),
        expected["kinds"]
            .as_array()
            .expect("exact acquisition roles")
            .iter()
            .take(acquisitions)
            .map(|kind| kind.as_str().expect("resource kind"))
            .collect::<Vec<_>>()
    );
    assert_eq!(actual.failures, usize::from(fault.is_some()));
    assert_eq!(
        usize::from(actual.reserve.is_some()),
        fault.map_or(1, |fault| number(&fault.expected["reserve_calls"]))
    );
}
impl Actual {
    fn observe(
        &mut self,
        fault: Option<&Fault>,
        record: &Value,
        metadata: &Value,
        expected: &Value,
    ) {
        match text(record, "event") {
            "host-malloc" | "host-free" | "bind-allocation" => self.observe_physical(record),
            "control-metadata" | "vec-metadata" => self.observe_metadata(record, metadata),
            "acquire" => self.acquire(record, expected),
            "release" => {
                let (id, kind) =
                    self.live.remove(text(record, "pointer")).expect("exact live acquired payload");
                assert_eq!(number(&record["id"]), id);
                assert_eq!(text(record, "kind"), kind);
                assert!(
                    self.physical.values().any(|(actual, _)| *actual == id),
                    "logical retirement observed at actual canonical free entry"
                );
                if id == 2 && fault.is_none() {
                    assert!(
                        self.physical.values().any(|(actual, _)| *actual == 4),
                        "real replacement allocation before old free"
                    );
                }
                Self::bytes(record, expected, id);
                self.released.push(id);
            }
            "reserve-entry" => {
                let pointer = text(record, "pointer");
                assert_eq!(self.live.get(pointer).expect("source Vec remains live").1, "Vec");
                assert!(self.reserve.replace(pointer.into()).is_none());
                for (id, kind) in [(1, "String"), (2, "Vec"), (3, "String")] {
                    assert!(
                        self.live.values().any(|actual| actual.0 == id && actual.1 == kind),
                        "original A/vector and affine RHS B live at reserve"
                    );
                }
                assert_eq!(record["length"], 1);
                assert_eq!(record["capacity"], 1);
                assert_eq!(record["required"], 2);
            }
            "failure" => {
                self.failures += 1;
                let fault = fault.expect("unexpected runtime fault");
                assert_eq!(text(record, "kind"), fault.kind);
                assert_eq!(number(&record["id"]), fault.ordinal);
                assert_eq!(text(record, "pointer"), "0");
                Self::bytes(
                    record,
                    expected,
                    if fault.kind == "allocation" { fault.ordinal } else { 0 },
                );
                let source = text(record, "source");
                if source == "0" {
                    assert_eq!(fault.kind, "allocation");
                    assert!(fault.ordinal <= 3);
                } else {
                    assert!(self.live.contains_key(source));
                }
            }
            _ => panic!("unexpected push/upgrade observer record"),
        }
    }
    fn acquire(&mut self, record: &Value, expected: &Value) {
        let pointer = text(record, "pointer");
        let id = number(&record["id"]);
        assert_ne!(address(pointer), 0);
        assert_eq!(self.bindings.remove(&id).as_deref(), Some(pointer));
        if text(record, "kind") != "String" {
            assert!(self.metadata.remove(pointer));
        }
        assert_eq!(id, self.kinds.len() + 1);
        let source = text(record, "source");
        if source != "0" {
            assert_eq!(self.reserve.as_deref(), Some(source));
            assert!(!self.live.contains_key(source));
            assert_eq!(self.all.get(source).expect("actual replaced source buffer").1, "Vec");
        }
        Self::bytes(record, expected, id);
        let kind = text(record, "kind").to_owned();
        assert!(self.all.insert(pointer.into(), (id, kind.clone())).is_none());
        assert!(self.live.insert(pointer.into(), (id, kind.clone())).is_none());
        self.kinds.push(kind);
    }
    fn bytes(record: &Value, expected: &Value, id: usize) {
        let hex = expected["string_acquisition_utf8_hex"][id.to_string()].as_str().unwrap_or("");
        assert_eq!(record["bytes_hex"], hex);
        assert_eq!(number(&record["length"]), hex.len() / 2);
    }
}
