//! Fixed expected effects and real-pointer identity validation; no compiler-derived oracle.
use super::authority;
use std::collections::{HashMap, HashSet};
#[derive(Clone, Debug)]
pub(super) struct Fault {
    pub(super) kind: String,
    pub(super) ordinal: usize,
    pub(super) releases: Vec<usize>,
}
pub(super) fn faults(case: &str) -> Vec<Fault> {
    authority::case(case)["faults"]
        .as_array()
        .expect("frozen fault cases")
        .iter()
        .map(|row| Fault {
            kind: row["kind"].as_str().expect("allocation/clone").into(),
            ordinal: number(&row["ordinal"]),
            releases: row["releases"]
                .as_array()
                .expect("frozen releases")
                .iter()
                .map(number)
                .collect(),
        })
        .collect()
}
pub(super) fn result(case: &str) -> i32 {
    authority::case(case)["observation"]
        .as_str()
        .expect("frozen observation")
        .strip_prefix("i32:")
        .expect("scalar i32 export")
        .parse()
        .expect("frozen scalar result")
}
pub(super) fn allocations(case: &str) -> usize {
    number(&authority::case(case)["allocations"])
}
fn number(value: &serde_json::Value) -> usize {
    usize::try_from(value.as_u64().expect("frozen nonnegative count")).expect("small count")
}
pub(super) fn check(case: &str, fault: Option<&Fault>, records: &[serde_json::Value]) {
    let row = authority::case(case);
    let releases = fault.map_or_else(
        || {
            row["string_releases"]
                .as_array()
                .expect("success drops")
                .iter()
                .map(number)
                .collect::<Vec<_>>()
        },
        |fault| fault.releases.clone(),
    );
    let mut actual = Actual::default();
    for record in records {
        actual.observe(record, &row, fault);
    }
    assert!(actual.owners.is_empty(), "every actual allocation released before return/trap");
    assert_eq!(actual.dropped, releases);
    assert_eq!(actual.addresses.len(), actual.dropped.len());
    assert_eq!(actual.failures, usize::from(fault.is_some()));
    let expected_tags = fault.map_or_else(
        || if allocations(case) == 0 { vec![] } else { vec![1, 2, 3, 3] },
        |fault| {
            if fault.kind == "allocation" && fault.ordinal == 1 {
                vec![4]
            } else {
                vec![1, if fault.kind == "allocation" { 4 } else { 5 }, 3]
            }
        },
    );
    assert_eq!(actual.tags, expected_tags);
    assert_eq!(
        actual.attempts,
        fault.map_or_else(
            || allocations(case),
            |fault| if fault.kind == "allocation" { fault.ordinal } else { 1 }
        )
    );
    assert_eq!(
        actual.clones,
        fault.map_or_else(
            || number(&row["clone_calls"]),
            |fault| usize::from(!(fault.kind == "allocation" && fault.ordinal == 1))
        )
    );
}

#[derive(Default)]
struct Actual {
    owners: HashMap<usize, String>,
    addresses: HashSet<String>,
    dropped: Vec<usize>,
    tags: Vec<usize>,
    failures: usize,
    attempts: usize,
    clones: usize,
}
impl Actual {
    fn observe(
        &mut self,
        record: &serde_json::Value,
        row: &serde_json::Value,
        fault: Option<&Fault>,
    ) {
        let tag = number(&record["tag"]);
        let id = number(&record["id"]);
        self.tags.push(tag);
        let pointer = record["identity"].as_str().expect("actual native pointer");
        let source = record["sourceIdentity"].as_str().expect("actual clone source pointer");
        let bytes = record["bytes_hex"].as_str().expect("actual observed UTF8 bytes");
        assert_eq!(record["length"], bytes.len() / 2);
        assert_eq!(
            bytes,
            row["string_utf8_hex"][if tag <= 3 { id - 1 } else { 0 }]
                .as_str()
                .expect("frozen actual byte value")
        );
        match tag {
            1 | 2 => {
                self.attempts += 1;
                assert_ne!(u64::from_str_radix(pointer, 16).expect("physical pointer"), 0);
                assert!(self.addresses.insert(pointer.to_owned()));
                assert_eq!(id, self.addresses.len());
                if tag == 1 {
                    assert_eq!(id, 1);
                    assert_eq!(source, "0");
                } else {
                    self.clones += 1;
                    assert_eq!(id, 2);
                    assert_eq!(Some(source), self.owners.get(&1).map(String::as_str));
                    assert_ne!(pointer, source);
                }
                assert!(self.owners.insert(id, pointer.into()).is_none());
            }
            3 => {
                assert_eq!(source, "0");
                assert_eq!(self.owners.remove(&id).as_deref(), Some(pointer));
                self.dropped.push(id);
            }
            4 | 5 => {
                self.failures += 1;
                let fault = fault.expect("unexpected failure");
                assert_eq!(tag, if fault.kind == "allocation" { 4 } else { 5 });
                assert_eq!(id, fault.ordinal);
                assert_eq!(pointer, "0");
                if tag == 4 {
                    self.attempts += 1;
                }
                if tag == 5 || source != "0" {
                    self.clones += 1;
                }
                assert_eq!(
                    source,
                    if fault.kind == "allocation" && fault.ordinal == 1 {
                        "0"
                    } else {
                        self.owners.get(&1).expect("Clone source still live").as_str()
                    }
                );
            }
            _ => panic!("record outside frozen observation protocol"),
        }
    }
}
