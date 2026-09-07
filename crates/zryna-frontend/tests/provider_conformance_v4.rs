//! Rust verification for the frozen provider-conformance snapshots.

use zryna_frontend::syntax_v4::{decode_snapshot, verify_snapshot};
use zryna_source::{SourceFileInput, SourceMap};

const POSITIVE_SOURCE: &str =
    include_str!("../../../tests/provider-conformance-v4/fixtures/positive.zry");
const POSITIVE_SNAPSHOT: &[u8] =
    include_bytes!("../../../tests/provider-conformance-v4/fixtures/positive.snapshot.json");
const ORDERING_A_SOURCE: &str =
    include_str!("../../../tests/provider-conformance-v4/fixtures/ordering-a.zry");
const ORDERING_Z_SOURCE: &str =
    include_str!("../../../tests/provider-conformance-v4/fixtures/ordering-z.zry");
const ORDERING_SNAPSHOT: &[u8] =
    include_bytes!("../../../tests/provider-conformance-v4/fixtures/ordering.snapshot.json");

fn source(path: &str, text: &str) -> SourceFileInput {
    SourceFileInput { path: path.to_owned(), text: text.to_owned() }
}

#[test]
fn canonical_snapshots_pass_the_rust_source_map_verifier() {
    let cases = [
        (vec![source("src/main.zry", POSITIVE_SOURCE)], POSITIVE_SNAPSHOT, vec!["src/main.zry"]),
        (
            vec![source("src/z.zry", ORDERING_Z_SOURCE), source("src/a.zry", ORDERING_A_SOURCE)],
            ORDERING_SNAPSHOT,
            vec!["src/a.zry", "src/z.zry"],
        ),
    ];
    for (inputs, bytes, expected_paths) in cases {
        let sources = SourceMap::build(inputs).expect("canonical source map");
        let raw = decode_snapshot(bytes).expect("closed protocol-v4 snapshot");
        let verified = verify_snapshot(raw, &sources).expect("source-bound protocol-v4 snapshot");
        assert!(verified.is_bound_to(&sources));
        assert_eq!(
            verified.files().iter().map(|file| file.path().as_str()).collect::<Vec<_>>(),
            expected_paths
        );
    }
}

#[test]
fn canonical_snapshot_rejects_different_source_bytes() {
    let changed = POSITIVE_SOURCE.replace("identity", "other_id");
    let sources = SourceMap::build(vec![source("src/main.zry", &changed)])
        .expect("bounded changed source map");
    let raw = decode_snapshot(POSITIVE_SNAPSHOT).expect("closed protocol-v4 snapshot");
    assert!(verify_snapshot(raw, &sources).is_err());
}
