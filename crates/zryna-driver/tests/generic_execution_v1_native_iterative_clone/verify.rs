//! Original Clone source IDs and destination-frame progress, frozen before Stage2 execution.
use std::{io::Read as _, os::unix::process::ExitStatusExt, path::Path};
#[path = "calls.rs"]
mod calls;
const TRACE_BYTES: u64 = 64 * 1024 * 1024;
fn bounded(path: &Path) -> Vec<u8> {
    let mut file = std::fs::File::open(path).expect("actual closed child trace");
    let expected = file.metadata().expect("trace metadata before allocation").len();
    assert!(expected <= TRACE_BYTES, "first-extra trace rejects before Vec reserve or read");
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(usize::try_from(expected).expect("bounded trace count"))
        .expect("fallible bounded trace storage");
    let mut chunk = [0u8; 4096];
    loop {
        let remaining = TRACE_BYTES
            .checked_sub(u64::try_from(bytes.len()).expect("trace length"))
            .expect("prior in-bound length");
        let request =
            chunk.len().min(usize::try_from(remaining + 1).expect("bounded first-extra read"));
        let count = file.read(&mut chunk[..request]).expect("bounded trace read");
        if count == 0 {
            break;
        }
        let end = bytes.len().checked_add(count).expect("trace length overflow");
        assert!(
            u64::try_from(end).expect("trace count") <= TRACE_BYTES,
            "concurrent first-extra rejects before extending or allocating"
        );
        bytes.try_reserve_exact(count).expect("fallible in-bound trace growth");
        bytes.extend_from_slice(&chunk[..count]);
    }
    bytes
}
fn rows(directory: &Path, label: &str) -> Vec<[u64; 8]> {
    let bytes = bounded(&directory.join(format!("{label}.trace.bin")));
    assert_eq!(bytes.len() % 64, 0);
    bytes
        .chunks_exact(64)
        .map(|row| {
            std::array::from_fn(|i| {
                u64::from_le_bytes(row[i * 8..i * 8 + 8].try_into().expect("exact u64 lane"))
            })
        })
        .collect()
}
fn ledger(rows: &[[u64; 8]], depth: u32, fault: bool) {
    use std::collections::{BTreeMap, BTreeSet};
    let d = u64::from(depth);
    let count = 2 * d + if fault { 1 } else { 2 };
    let acquired = rows.iter().filter(|row| row[0] == 2).map(|row| row[2]).collect::<Vec<_>>();
    assert_eq!(acquired, (1..=count).collect::<Vec<_>>(), "no carrier or failed-leaf allocation");
    let expected = (d + 2..=count).rev().chain(1..=d + 1).collect::<Vec<_>>();
    let released = rows.iter().filter(|row| row[0] == 3).map(|row| row[2]).collect::<Vec<_>>();
    assert_eq!(released, expected, "cloned prefix rollback/drop before original source drop");
    let owner = rows.iter().find(|row| row[0] == 1).expect("actual source call")[6];
    let mut live = BTreeMap::new();
    let mut pointers = BTreeSet::new();
    for row in rows {
        if matches!(row[0], 1..=4 | 6) {
            assert_eq!(row[5], 1);
            assert_ne!(owner, 0);
            assert_eq!(row[6], owner, "one actual retained TLS owner per entry");
            assert!(row[4] <= row[7]);
        }
        if row[0] == 2 {
            assert_ne!(row[3], 0);
            assert!(pointers.insert(row[3]), "fresh live allocation pointer");
            assert!(live.insert(row[2], row[3]).is_none());
        } else if row[0] == 3 {
            assert_eq!(live.remove(&row[2]), Some(row[3]), "free exact acquired pointer once");
            assert!(pointers.remove(&row[3]));
            let id = row[2];
            let used = if id <= d + 1 {
                d + 1 - id
            } else if id == 2 * d + 2 {
                d
            } else {
                id - (d + 2)
            };
            assert_eq!(row[4], used, "depth{depth}/free{id}: distinct pending other owners");
            assert!(row[4] < row[7], "free owns no pending frame");
        }
    }
    assert!(live.is_empty() && pointers.is_empty());
}
pub(super) fn check(
    directory: &Path,
    label: &str,
    depth: u32,
    fault: bool,
    output: &std::process::Output,
) {
    let actual = rows(directory, label);
    // Child trace/stdout/stderr/status were retained before entering this verifier.
    calls::check(&actual, depth, fault);
    ledger(&actual, depth, fault);
    let first = actual.first().expect("entry snapshot");
    assert_eq!((first[0], first[1], first[2], first[3]), (7, 0, 0, u64::from(depth)));
    assert_eq!((first[4], first[5], first[6], first[7]), (0, 0, 0, 0));
    let last = actual.last().expect("completed cleanup snapshot");
    if fault {
        assert_eq!(output.status.signal(), Some(libc::SIGILL));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        assert_eq!((last[0], last[2]), (8, u64::try_from(libc::SIGILL).expect("signal")));
    } else {
        assert!(
            output.status.success(),
            "{label}: {:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"result:7\n");
        assert!(output.stderr.is_empty());
        assert_eq!((last[0], last[2], last[3]), (5, 7, 2 * u64::from(depth) + 2));
    }
    assert_eq!((last[4], last[5], last[6], last[7]), (0, 0, 0, 0));
}
