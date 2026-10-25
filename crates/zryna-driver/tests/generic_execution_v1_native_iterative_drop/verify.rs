//! Independent frozen source formulas over actual malloc/free identities and real TLS snapshots.
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
fn identities(rows: &[[u64; 8]], expected: &[u64], allocations: u64, depth: u32, case: &str) {
    let acquired = rows.iter().filter(|r| r[0] == 2).map(|r| r[2]).collect::<Vec<_>>();
    assert_eq!(acquired, (1..=allocations).collect::<Vec<_>>());
    let released = rows.iter().filter(|r| r[0] == 3).map(|r| r[2]).collect::<Vec<_>>();
    assert_eq!(released, expected);
    let d = u64::from(depth);
    for row in rows.iter().filter(|r| r[0] == 3) {
        let id = row[2];
        assert!((1..=allocations).contains(&id));
        let used =
            if case == "shared-weak-recursive" { d + 2 - id } else { (d + 1).saturating_sub(id) };
        assert_eq!(row[4], used, "{case}/depth{depth}/free{id}: exact distinct live ancestors");
    }
    for row in rows.iter().filter(|r| matches!(r[0], 1..=4 | 6)) {
        assert_eq!(row[5], 1, "actual call must be inside one real TLS lifecycle");
        assert_ne!(row[6], 0);
        assert!(row[4] <= row[7], "frames remain within actual live positive owners");
    }
    let pointer_owner = rows.iter().find(|r| r[0] == 1).expect("actual canonical call")[6];
    assert!(rows.iter().filter(|r| matches!(r[0], 1..=4 | 6)).all(|r| r[6] == pointer_owner));
}
fn idle(row: &[u64; 8]) {
    assert_eq!((row[4], row[5], row[6], row[7]), (0, 0, 0, 0));
}
pub(super) fn success(
    directory: &Path,
    label: &str,
    case: &str,
    depth: u32,
    output: &std::process::Output,
) {
    let actual = rows(directory, label);
    calls::check(&actual, case, depth, false);
    assert!(
        output.status.success(),
        "actual native observation {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"result:7\n");
    let d = u64::from(depth);
    let allocations = d + if case == "drop-only" {
        1
    } else if case == "controlled-cleanup" {
        3
    } else {
        2
    };
    let mut expected = Vec::new();
    if case == "controlled-cleanup" {
        expected.extend([d + 3, d + 2]);
    }
    expected.extend(1..=if case == "controlled-cleanup" { d + 1 } else { allocations });
    identities(&actual, &expected, allocations, depth, case);
    let done = actual.last().expect("done snapshot");
    assert_eq!((done[0], done[2], done[3]), (5, 7, allocations));
    idle(done);
    if case == "shared-weak-recursive" {
        let begin =
            actual.iter().filter(|r| r[0] == 6 && r[1] == 14).map(|r| r[2]).collect::<Vec<_>>();
        assert_eq!(begin, [0, 1], "alias before lastStrong payload traversal");
        let weak =
            actual.iter().filter(|r| r[0] == 6 && r[1] == 16).map(|r| r[2]).collect::<Vec<_>>();
        assert_eq!(weak, [0, 0], "both user weak counts end while strong owner remains");
    }
}
pub(super) fn fault(directory: &Path, label: &str, depth: u32, output: &std::process::Output) {
    let actual = rows(directory, label);
    calls::check(&actual, "controlled-cleanup", depth, true);
    assert_eq!(output.status.signal(), Some(libc::SIGILL));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let d = u64::from(depth);
    let expected = std::iter::once(d + 2).chain(1..=d + 1).collect::<Vec<_>>();
    identities(&actual, &expected, d + 2, depth, "controlled-cleanup");
    assert!(actual.iter().any(|r| r[0] == 6 && r[1] == 4 && r[2] == 1 && r[3] == 1));
    let last = actual.last().expect("actual SIGILL snapshot after cleanup");
    assert_eq!((last[0], last[2]), (8, u64::try_from(libc::SIGILL).expect("signal")));
    idle(last);
}
pub(super) fn hostile(directory: &Path, label: &str, seed: u32, output: &std::process::Output) {
    let actual = rows(directory, label);
    assert_eq!(output.status.signal(), Some(libc::SIGILL), "invalid TLS is an ABI failure");
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert_eq!(actual.first().expect("entry seed")[2], u64::from(seed));
    assert!(
        !actual.iter().any(|r| matches!(r[0], 1..=6)),
        "reject before canonical/source effects"
    );
    let last = actual.last().expect("actual ABI rejection signal");
    assert_eq!(last[0], 8);
    assert_eq!(last[7], 0);
}
