//! Frozen expected effects versus actual canonical-runtime pointer/count/UTF8 observations.
use serde_json::Value;
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
pub(super) struct Fault {
    pub(super) kind: String,
    pub(super) ordinal: usize,
    row: Value,
}
pub(super) fn row(case: &str) -> Value {
    let bytes = include_bytes!("RUNTIME_EXPECTED.json");
    assert_eq!(
        super::emit::digest(bytes),
        "816c5d94d7dc20a8520c138f3dcaa9f16ac5d499d573f1fbe6bee0d25cdf6e8d"
    );
    let expected: Value = serde_json::from_slice(bytes).expect("pre-implementation native oracle");
    expected["cases"]
        .as_array()
        .expect("four originals")
        .iter()
        .find(|row| row["case"] == case)
        .expect("frozen case")
        .clone()
}
pub(super) fn faults(case: &str) -> Vec<Fault> {
    let expected = row(case);
    expected["faults"]
        .as_array()
        .expect("allocation/StringClone faults")
        .iter()
        .chain(expected["count_faults"].as_array().expect("declared status route controls"))
        .map(|row| Fault {
            kind: string(row, "kind").into(),
            ordinal: number(&row["ordinal"]),
            row: row.clone(),
        })
        .collect()
}
fn number(value: &Value) -> usize {
    usize::try_from(value.as_u64().expect("nonnegative bounded value"))
        .expect("small fixture integer")
}
fn string<'a>(value: &'a Value, name: &str) -> &'a str {
    value[name].as_str().expect("actual trace string")
}
#[derive(Default)]
struct Actual {
    live: HashMap<String, (usize, String)>,
    identities: HashSet<String>,
    released: Vec<usize>,
    acquired: Vec<String>,
    counts: Vec<String>,
    count_state: Option<(u64, u64)>,
    failures: usize,
    clones: usize,
    vector_metadata: HashSet<String>,
    control_metadata: HashSet<String>,
}
pub(super) fn check(case: &str, fault: Option<&Fault>, records: &[Value], metadata: &Value) {
    let expected = row(case);
    let mut actual = Actual::default();
    for record in records {
        actual.observe(record, &expected, fault, metadata);
    }
    assert!(actual.live.is_empty(), "every actual allocation released before return/trap");
    assert_eq!(actual.acquired.len(), actual.released.len());
    let release = fault.map_or(&expected["releases"], |fault| &fault.row["releases"]);
    assert_eq!(
        actual.released,
        release.as_array().expect("frozen reverse cleanup").iter().map(number).collect::<Vec<_>>()
    );
    let acquisitions = expected["acquisitions"].as_array().expect("frozen actual allocation order");
    let count = fault.map_or(acquisitions.len(), |fault| match fault.kind.as_str() {
        "allocation" => fault.ordinal - 1,
        "string-clone" => 3 + fault.ordinal,
        _ => 2,
    });
    assert_eq!(actual.acquired.len(), count);
    let kinds = acquisitions
        .iter()
        .take(count)
        .map(|value| {
            let name = value.as_str().expect("resource role");
            if name.contains("String") {
                "String"
            } else if name.contains("Vec") {
                "Vec"
            } else {
                "control"
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(actual.acquired.iter().map(String::as_str).collect::<Vec<_>>(), kinds);
    assert_eq!(actual.failures, usize::from(fault.is_some()));
    let clones = fault.map_or_else(
        || number(&expected["string_clone_calls"]),
        |fault| match fault.kind.as_str() {
            "string-clone" => fault.ordinal,
            "allocation" if case == "vec-owned-prepare" && fault.ordinal >= 5 => fault.ordinal - 4,
            _ => 0,
        },
    );
    assert_eq!(actual.clones, clones);
    let counts =
        if case != "shared-weak-count" || fault.is_some_and(|fault| fault.kind == "allocation") {
            vec![]
        } else {
            fault
                .map_or(&expected["count_events"], |fault| &fault.row["count_events"])
                .as_array()
                .expect("exact count/payload order")
                .iter()
                .map(|value| value.as_str().expect("count event").to_owned())
                .collect::<Vec<_>>()
        };
    assert_eq!(actual.counts, counts);
    assert!(
        actual.control_metadata.is_empty(),
        "every control metadata record has an actual allocation"
    );
    assert!(
        actual.vector_metadata.is_empty(),
        "every metadata record belongs to a real buffer acquisition"
    );
}
impl Actual {
    fn observe(
        &mut self,
        record: &Value,
        expected: &Value,
        fault: Option<&Fault>,
        metadata: &Value,
    ) {
        match string(record, "event") {
            "acquire" => self.acquire(record, expected),
            "release" => {
                let pointer = string(record, "pointer");
                let (id, kind) = self.live.remove(pointer).expect("exact live acquired pointer");
                assert_eq!(number(&record["id"]), id);
                assert_eq!(string(record, "kind"), kind);
                Self::bytes(record, expected, id);
                self.released.push(id);
            }
            "clone-entry" => {
                self.clones += 1;
                assert_eq!(number(&record["id"]), self.clones);
                let pointer = string(record, "pointer");
                assert_eq!(pointer, string(record, "source"));
                let (id, kind) =
                    self.live.get(pointer).expect("actual String Clone source still live");
                assert_eq!(kind, "String");
                Self::bytes(record, expected, *id);
            }
            "failure" => self.failure(record, expected, fault.expect("unexpected failure")),
            "count" => self.count(record),
            "payload-drop" => {
                let control = self
                    .live
                    .get(string(record, "source"))
                    .expect("actual control retained until payload drop");
                assert_eq!(control.1, "control");
                assert_eq!(self.count_state, Some((0, 1)));
                Self::bytes(record, expected, number(&record["id"]));
                self.counts.push("payloadDrop".into());
            }
            "control-metadata" => {
                assert!(
                    metadata["control_layouts"]
                        .as_array()
                        .expect("same NEWseal control metadata")
                        .iter()
                        .any(|row| row["size"] == record["size"]
                            && row["alignment"] == record["alignment"])
                );
                assert!(self.control_metadata.insert(string(record, "pointer").into()));
            }
            "vec-metadata" => {
                let actual = metadata["element_layouts"]
                    .as_array()
                    .expect("same new-seal runtime element inventory")
                    .iter()
                    .find(|row| row["element"] == record["element"])
                    .expect("exact declared element ID");
                assert_eq!(record["stride"], actual["stride"]);
                assert_eq!(record["alignment"], actual["alignment"]);
                assert_eq!(
                    number(&record["required"]),
                    if expected["case"] == "vec-copy" { 1 } else { 2 }
                );
                assert!(
                    record["capacity"].as_u64().expect("actual capacity")
                        >= record["required"].as_u64().expect("required length")
                );
                assert!(self.vector_metadata.insert(string(record, "pointer").into()));
            }
            _ => panic!("unexpected canonical-runtime observer record"),
        }
    }
    fn bytes(record: &Value, expected: &Value, id: usize) {
        let hex = expected["string_acquisition_utf8_hex"][id.to_string()].as_str().unwrap_or("");
        assert_eq!(record["bytes_hex"], hex);
        assert_eq!(number(&record["length"]), hex.len() / 2);
    }
    fn acquire(&mut self, record: &Value, expected: &Value) {
        let pointer = string(record, "pointer");
        let kind = string(record, "kind");
        assert_ne!(u64::from_str_radix(pointer, 16).expect("actual native address"), 0);
        assert!(self.identities.insert(pointer.into()));
        let id = self.identities.len();
        assert_eq!(number(&record["id"]), id);
        let source = string(record, "source");
        if source != "0" {
            let (source_id, source_kind) =
                self.live.get(source).expect("actual Clone source allocation remains live");
            assert_eq!(source_kind, "String");
            assert_eq!(kind, "String");
            assert_ne!(pointer, source);
            assert_eq!(
                expected["string_acquisition_utf8_hex"][source_id.to_string()],
                expected["string_acquisition_utf8_hex"][id.to_string()]
            );
        }
        if kind == "Vec" {
            assert!(self.vector_metadata.remove(pointer));
        }
        if kind == "control" {
            assert!(self.control_metadata.remove(pointer));
        }
        Self::bytes(record, expected, id);
        assert!(self.live.insert(pointer.into(), (id, kind.into())).is_none());
        self.acquired.push(kind.into());
    }
    fn failure(&mut self, record: &Value, expected: &Value, fault: &Fault) {
        self.failures += 1;
        assert_eq!(string(record, "kind"), fault.kind);
        assert_eq!(number(&record["id"]), fault.ordinal);
        assert_eq!(string(record, "pointer"), "0");
        let source = string(record, "source");
        if source != "0" {
            let (_, kind) = self.live.get(source).expect("failed action source still live");
            assert_eq!(kind, if fault.kind.ends_with("-status") { "control" } else { "String" });
        }
        let id = if fault.kind == "allocation" {
            fault.ordinal
        } else if fault.kind == "string-clone" {
            4 + fault.ordinal
        } else {
            0
        };
        Self::bytes(record, expected, id);
    }
    fn count(&mut self, record: &Value) {
        let (_, kind) = self.live.get(string(record, "pointer")).expect("actual retained control");
        assert_eq!(kind, "control");
        let before = (
            record["beforeStrong"].as_u64().expect("actual strong"),
            record["beforeWeak"].as_u64().expect("actual weak"),
        );
        assert_eq!(before, self.count_state.unwrap_or((1, 1)));
        let after = (
            record["afterStrong"].as_u64().expect("actual strong"),
            record["afterWeak"].as_u64().expect("actual weak"),
        );
        let name = string(record, "name");
        let expected = match name {
            "strongClone" => (before.0 + 1, before.1),
            "weakDowngrade" | "weakClone" => (before.0, before.1 + 1),
            "weakRelease" | "strongReleaseFinish:deallocated" | "strongReleaseFinish:retained" => {
                (before.0, before.1 - 1)
            }
            "strongReleaseBegin:last" | "strongReleaseBegin:notLast" => (before.0 - 1, before.1),
            name if name.ends_with(":failure") => before,
            _ => panic!("unknown count action"),
        };
        assert_eq!(after, expected);
        assert_eq!(record["status"], if name.ends_with(":failure") { 3 } else { 0 });
        if name == "strongReleaseBegin:last" {
            assert_eq!(after.0, 0);
        }
        if name == "strongReleaseBegin:notLast" {
            assert_ne!(after.0, 0);
        }
        if name == "strongReleaseFinish:deallocated" {
            assert_eq!(after, (0, 0));
        }
        self.count_state = Some(after);
        self.counts.push(name.into());
    }
}
