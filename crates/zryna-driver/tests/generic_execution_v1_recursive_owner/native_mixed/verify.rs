//! Exact full physical ledger, real TLS snapshots and frozen cleanup identities.
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read as _,
    os::unix::process::ExitStatusExt,
    path::Path,
};
fn bounded(path: &Path, limit: u64) -> Vec<u8> {
    let mut file = std::fs::File::open(path).expect("actual closed child trace");
    let length = file.metadata().expect("trace metadata").len();
    assert!(length <= limit, "before reserve/read");
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(usize::try_from(length).expect("bounded length"))
        .expect("trace storage");
    let mut scratch = [0u8; 4096];
    loop {
        let remaining = limit - u64::try_from(bytes.len()).expect("bounded trace");
        let request = scratch.len().min(usize::try_from(remaining + 1).expect("first extra"));
        let count = file.read(&mut scratch[..request]).expect("bounded read");
        if count == 0 {
            break;
        }
        assert!(u64::try_from(bytes.len() + count).expect("bounded total") <= limit);
        bytes.try_reserve_exact(count).expect("in-bound growth");
        bytes.extend_from_slice(&scratch[..count]);
    }
    bytes
}
pub(super) fn check(
    directory: &Path,
    label: &str,
    oracle: &Value,
    addendum: &Value,
    fault: Option<&Value>,
    output: &std::process::Output,
) {
    // Both complete trace files and stdout/stderr/status already exist before any assertions.
    let raw = bounded(&directory.join(format!("{label}.bin")), 64 * 1024 * 1024);
    let json = bounded(&directory.join(format!("{label}.jsonl")), 65_536);
    assert_eq!(raw.len() % 64, 0);
    let rows = raw
        .chunks_exact(64)
        .map(|row| {
            std::array::from_fn(|index| {
                u64::from_le_bytes(row[index * 8..index * 8 + 8].try_into().expect("word"))
            })
        })
        .collect::<Vec<[u64; 8]>>();
    let expected = fault.unwrap_or(&oracle["pristine"]);
    super::calls::check(&rows, super::expected::calls(addendum, fault), fault);
    ledger(&rows, expected);
    let records = std::str::from_utf8(&json)
        .expect("actual UTF8 JSON")
        .lines()
        .map(|line| serde_json::from_str(line).expect("complete actual JSON record"))
        .collect::<Vec<Value>>();
    super::projection::check(&records, oracle, expected, fault);
    let first = rows.first().expect("entry lifecycle");
    assert_eq!(first[0], 7);
    assert_eq!(
        (first[2], first[3]),
        fault.map_or((0, 0), |fault| (
            if fault["kind"] == "Allocate" { 2 } else { 6 },
            fault["ordinal"].as_u64().expect("ordinal")
        ))
    );
    assert_eq!((first[4], first[5], first[6], first[7]), (0, 0, 0, 0));
    let last = rows.last().expect("clean return/trap snapshot");
    assert_eq!((last[4], last[5], last[6], last[7]), (0, 0, 0, 0));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    if fault.is_some() {
        assert_eq!(output.status.signal(), Some(libc::SIGILL));
        assert!(output.stdout.is_empty());
        assert_eq!((last[0], last[2]), (8, u64::try_from(libc::SIGILL).expect("signal")));
    } else {
        assert!(output.status.success());
        assert_eq!(output.stdout, b"result:97\n");
        assert_eq!((last[0], last[2], last[3]), (5, 97, 26));
    }
}
fn ledger(rows: &[[u64; 8]], expected: &Value) {
    let expected_ids = |key: &str| {
        expected[key]
            .as_array()
            .expect("source identities")
            .iter()
            .map(|id| id.as_u64().expect("identity"))
            .collect::<Vec<_>>()
    };
    let acquires = rows.iter().filter(|row| row[0] == 2).map(|row| row[2]).collect::<Vec<_>>();
    assert_eq!(
        acquires,
        expected_ids(if expected.get("acquired_ids").is_some() {
            "acquired_ids"
        } else {
            "acquisition_ids"
        })
    );
    assert_eq!(
        rows.iter().filter(|row| row[0] == 3).map(|row| row[2]).collect::<Vec<_>>(),
        expected_ids("release_ids")
    );
    let owner = rows.iter().find(|row| row[0] == 1).expect("actual canonical entry")[6];
    assert_ne!(owner, 0);
    let mut live = BTreeMap::new();
    let mut pointers = BTreeSet::new();
    for row in rows {
        if matches!(row[0], 1..=4 | 6) {
            assert_eq!((row[5], row[6]), (1, owner));
            assert!(row[4] <= row[7]);
        }
        if row[0] == 2 {
            assert_ne!(row[3], 0);
            assert!(pointers.insert(row[3]));
            assert!(live.insert(row[2], row[3]).is_none());
        }
        if row[0] == 3 {
            assert_eq!(live.remove(&row[2]), Some(row[3]));
            assert!(pointers.remove(&row[3]));
            assert!(row[4] < row[7]);
        }
    }
    assert!(live.is_empty() && pointers.is_empty());
}
