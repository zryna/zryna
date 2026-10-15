//! Fixed upstream source inputs, rehashed independently of the acquisition receipt.
use super::*;
use std::{
    fs,
    io::Read as _,
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::Path,
};

const MANIFEST: &str = include_str!("../../../../../../../../../scripts/m5/sqlite_fixture.json");
pub(super) struct Sources {
    pub(super) source: Vec<u8>,
    pub(super) header: Vec<u8>,
}

pub(super) fn capture() -> Sources {
    let manifest: serde_json::Value =
        serde_json::from_str(MANIFEST).expect("fixed source manifest");
    assert_eq!(manifest["format"], "zryna.sqlite-source-fixture.v1");
    assert_eq!(manifest["version"], "3.54.0");
    assert_eq!(manifest["versionNumber"], 3_054_000);
    assert_eq!(manifest["files"].as_object().expect("fixed source files").len(), 2);
    let root = std::env::var_os("ZRYNA_SQLITE_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .expect("required pinned SQLite source acquisition setup");
    assert!(root.is_absolute());
    for ancestor in root.ancestors() {
        assert!(fs::symlink_metadata(ancestor).expect("source ancestor").is_dir());
    }
    let mut selected = Vec::new();
    for (name, maximum) in [("sqlite3.c", 16 * 1024 * 1024), ("sqlite3.h", 2 * 1024 * 1024)] {
        let expected = &manifest["files"][name];
        assert_eq!(expected["maxBytes"], maximum);
        let bytes = bounded_read(&root.join(name), maximum);
        assert_eq!(
            u64::try_from(bytes.len()).expect("source size"),
            expected["bytes"].as_u64().expect("fixed size")
        );
        assert_eq!(hex(&sha(&bytes)), expected["sha256"].as_str().expect("fixed SHA256"));
        selected.push(bytes);
    }
    let header = selected.pop().expect("fixed header");
    let source = selected.pop().expect("fixed amalgamation");
    Sources { source, header }
}

pub(super) fn bounded_read(path: &Path, maximum: usize) -> Vec<u8> {
    let before = fs::symlink_metadata(path).expect("actual regular bounded input");
    assert!(before.is_file() && before.len() > 0);
    assert!(before.len() <= u64::try_from(maximum).expect("input cap"));
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .expect("no-follow input");
    let opened = file.metadata().expect("opened input identity");
    assert_eq!(identity(&before), identity(&opened));
    let mut bytes = Vec::with_capacity(usize::try_from(before.len()).expect("bounded length"));
    file.by_ref()
        .take(u64::try_from(maximum).expect("read cap") + 1)
        .read_to_end(&mut bytes)
        .expect("bounded actual read");
    let after = fs::symlink_metadata(path).expect("unchanged named input");
    assert_eq!(identity(&before), identity(&file.metadata().expect("unchanged open input")));
    assert_eq!(identity(&before), identity(&after));
    assert_eq!(u64::try_from(bytes.len()).expect("bounded length"), before.len());
    bytes
}

fn identity(value: &fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        value.dev(),
        value.ino(),
        value.len(),
        value.mtime(),
        value.mtime_nsec(),
        value.ctime(),
        value.ctime_nsec(),
    )
}
