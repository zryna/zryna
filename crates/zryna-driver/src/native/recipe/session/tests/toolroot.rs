use sha2::{Digest as _, Sha256};

use super::super::super::archive::{Metadata, RootEntry, RootKind};
use super::super::toolroot;
use super::support;

#[test]
fn unlisted_executable_data_and_tool_body_substitution_refuse_and_recover() {
    let root = support::root();
    let original = toolroot::identity(&root).expect("operator data identity only");
    let mut extra = root.clone();
    extra.entries.insert(
        "/tool/extra".into(),
        RootEntry {
            metadata: Metadata { uid: 0, gid: 0, mode: 0o555 },
            kind: RootKind::File { bytes: 4, sha256: Sha256::digest(b"\x7fELF").into() },
        },
    );
    assert!(toolroot::identity(&extra).is_err());
    let mut body = root.clone();
    body.entries.get_mut("/tool/bin/gcc").expect("tool").kind =
        RootKind::File { bytes: 1, sha256: [0; 32] };
    assert!(toolroot::identity(&body).is_err());
    let mut tool = root.clone();
    tool.tools.insert("/tool/bin/gcc".into(), "0".repeat(64));
    assert!(toolroot::identity(&tool).is_err());
    assert_eq!(toolroot::identity(&root).expect("pristine recovery"), original);
}

#[test]
fn unsafe_path_owner_permissions_and_unapproved_loader_aliases_refuse() {
    let root = support::root();
    for (name, uid, mode) in [
        ("/../escape", 0, 0o755),
        ("/extra", 65534, 0o755),
        ("/tool", 0, 0o777),
        ("/input", 0, 0o755),
    ] {
        let mut bad = root.clone();
        bad.entries.insert(
            name.into(),
            RootEntry { metadata: Metadata { uid, gid: 0, mode }, kind: RootKind::Directory },
        );
        assert!(toolroot::identity(&bad).is_err());
    }
    let mut alias = root.clone();
    alias.entries.insert(
        "/unexpected-loader".into(),
        RootEntry {
            metadata: Metadata { uid: 0, gid: 0, mode: 0o777 },
            kind: RootKind::Symlink { target: "/lib64/ld-linux-x86-64.so.2".into() },
        },
    );
    assert!(toolroot::identity(&alias).is_err());
    toolroot::identity(&root).expect("valid data recovery");
}
