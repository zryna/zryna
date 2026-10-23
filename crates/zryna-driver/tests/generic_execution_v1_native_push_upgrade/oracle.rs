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
        "760338945f7103ed102fb4aca8d2aaf3aae9704cada1c313be433b376c60610f"
    );
    let expected: Value = serde_json::from_slice(bytes).expect("frozen native push/upgrade oracle");
    expected["cases"]
        .as_array()
        .expect("three original cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("original case")
        .clone()
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
    count_events: Vec<String>,
    counts: Option<(u64, u64)>,
    metadata: HashSet<String>,
    reserve: Option<String>,
    failures: usize,
    mallocs: usize,
}
pub(super) fn check(case: &str, fault: Option<&Fault>, records: &[Value], metadata: &Value) {
    let expected = row(case);
    let mut actual = Actual::default();
    for record in records {
        actual.observe(case, fault, record, metadata);
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
    assert!(
        actual.kinds.iter().all(|kind| kind == if case == "vec-push" { "Vec" } else { "control" })
    );
    let counts = fault.map_or(&expected["count_events"], |fault| {
        fault.expected.get("count_events").unwrap_or(&expected["count_events"])
    });
    assert_eq!(
        actual.count_events,
        counts
            .as_array()
            .expect("fixed count order")
            .iter()
            .map(|event| event.as_str().expect("event").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(actual.failures, usize::from(fault.is_some()));
    assert_eq!(
        usize::from(actual.reserve.is_some()),
        usize::from(
            case == "vec-push"
                && !fault.is_some_and(|fault| fault.kind == "allocation" && fault.ordinal == 1)
        )
    );
}
impl Actual {
    fn observe(&mut self, case: &str, fault: Option<&Fault>, record: &Value, metadata: &Value) {
        match text(record, "event") {
            "host-malloc" | "host-free" | "bind-allocation" => self.observe_physical(record),
            "control-metadata" | "vec-metadata" => self.observe_metadata(record, metadata),
            "acquire" => self.acquire(record),
            "release" => {
                let (id, kind) =
                    self.live.remove(text(record, "pointer")).expect("exact live acquired payload");
                assert_eq!(number(&record["id"]), id);
                assert_eq!(text(record, "kind"), kind);
                assert!(
                    self.physical.values().any(|(actual, _)| *actual == id),
                    "logical retirement observed at actual canonical free entry"
                );
                if case == "vec-push" && id == 1 && fault.is_none() {
                    assert!(
                        self.physical.values().any(|(actual, _)| *actual == 2),
                        "real replacement allocation before old free"
                    );
                }
                self.released.push(id);
            }
            "reserve-entry" => {
                assert_eq!(case, "vec-push");
                let pointer = text(record, "pointer");
                assert_eq!(self.live.get(pointer).expect("source Vec remains live").1, "Vec");
                assert!(self.reserve.replace(pointer.into()).is_none());
                assert_eq!(record["length"], 1);
                assert_eq!(record["capacity"], 1);
                assert_eq!(record["required"], 2);
            }
            "count" => self.count(record),
            "failure" => {
                self.failures += 1;
                let fault = fault.expect("unexpected runtime fault");
                assert_eq!(text(record, "kind"), fault.kind);
                assert_eq!(number(&record["id"]), fault.ordinal);
                assert_eq!(text(record, "pointer"), "0");
                let source = text(record, "source");
                if source == "0" {
                    assert_eq!(fault.kind, "allocation");
                    assert_eq!(fault.ordinal, 1);
                } else {
                    assert!(self.live.contains_key(source));
                }
            }
            _ => panic!("unexpected push/upgrade observer record"),
        }
    }
    fn acquire(&mut self, record: &Value) {
        let pointer = text(record, "pointer");
        let id = number(&record["id"]);
        assert_ne!(address(pointer), 0);
        assert_eq!(self.bindings.remove(&id).as_deref(), Some(pointer));
        assert!(self.metadata.remove(pointer));
        assert_eq!(id, self.kinds.len() + 1);
        let source = text(record, "source");
        if source != "0" {
            assert_eq!(self.reserve.as_deref(), Some(source));
            assert!(!self.live.contains_key(source));
            assert_eq!(self.all.get(source).expect("actual replaced source buffer").1, "Vec");
        }
        assert_eq!(record["length"], 0);
        assert_eq!(record["bytes_hex"], "");
        let kind = text(record, "kind").to_owned();
        assert!(self.all.insert(pointer.into(), (id, kind.clone())).is_none());
        assert!(self.live.insert(pointer.into(), (id, kind.clone())).is_none());
        self.kinds.push(kind);
    }
    fn count(&mut self, record: &Value) {
        let (id, kind) =
            self.all.get(text(record, "pointer")).expect("same actual control identity");
        assert_eq!(kind, "control");
        let name = text(record, "name");
        if !self.live.contains_key(text(record, "pointer")) {
            assert!(self.freed.contains(id));
            assert!(matches!(name, "weakRelease" | "strongReleaseFinish:deallocated"));
        }
        let before = (
            record["beforeStrong"].as_u64().expect("actual strong"),
            record["beforeWeak"].as_u64().expect("actual weak"),
        );
        assert_eq!(before, self.counts.unwrap_or((1, 1)));
        let after = (
            record["afterStrong"].as_u64().expect("actual strong"),
            record["afterWeak"].as_u64().expect("actual weak"),
        );
        let expected = match name {
            "weakDowngrade" => (before.0, before.1 + 1),
            "weakUpgrade:live" => (before.0 + 1, before.1),
            "weakUpgrade:expired" => (before.0, before.1),
            "weakRelease" | "strongReleaseFinish:deallocated" | "strongReleaseFinish:retained" => {
                (before.0, before.1 - 1)
            }
            "strongReleaseBegin:last" | "strongReleaseBegin:notLast" => (before.0 - 1, before.1),
            name if name.ends_with(":failure") => before,
            _ => panic!("unexpected exact count action"),
        };
        assert_eq!(after, expected);
        assert_eq!(
            record["status"],
            if name.ends_with(":failure") {
                3
            } else if name == "weakUpgrade:expired" {
                5
            } else {
                0
            }
        );
        if name == "weakUpgrade:expired" {
            assert_eq!(before.0, 0);
        }
        if name == "strongReleaseBegin:last" {
            assert_eq!(after.0, 0);
        }
        if name == "strongReleaseBegin:notLast" {
            assert_ne!(after.0, 0);
        }
        if name == "strongReleaseFinish:deallocated" {
            assert_eq!(after, (0, 0));
        }
        self.counts = Some(after);
        self.count_events.push(name.into());
    }
}
