use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::native::recipe::archive::{Error, Metadata, RootEntry, RootKind, extract_root, header};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn metadata(mode: u32) -> Metadata {
    Metadata { uid: 0, gid: 0, mode }
}
fn checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}
fn record(name: &str, body: &[u8], kind: u8, mode: [u8; 8], target: &str) -> Vec<u8> {
    let mut block = header::encode(name, body.len(), kind == b'5').expect("header");
    block[100..108].copy_from_slice(&mode);
    block[156] = kind;
    block[157..157 + target.len()].copy_from_slice(target.as_bytes());
    checksum(&mut block);
    let mut out = block.to_vec();
    out.extend_from_slice(body);
    out.resize(512 + super::super::padded(body.len()).expect("small body"), 0);
    out
}
fn fixture() -> (Vec<u8>, BTreeMap<String, RootEntry>) {
    let mut tar = record("/", b"", b'5', *b"0000755\0", "");
    tar.extend(record("/tool/", b"", b'5', *b"0000755\0", ""));
    tar.extend(record("/tool/gcc", b"ELF body", b'0', *b"0000555\0", ""));
    tar.extend(record("/loader", b"", b'2', *b"0000777\0", "/tool/gcc"));
    tar.resize(tar.len() + 1024, 0);
    let expected = BTreeMap::from([
        (String::new(), RootEntry { metadata: metadata(0o755), kind: RootKind::Directory }),
        ("/tool".to_owned(), RootEntry { metadata: metadata(0o755), kind: RootKind::Directory }),
        (
            "/tool/gcc".to_owned(),
            RootEntry {
                metadata: metadata(0o555),
                kind: RootKind::File { bytes: 8, sha256: Sha256::digest(b"ELF body").into() },
            },
        ),
        (
            "/loader".to_owned(),
            RootEntry {
                metadata: metadata(0o777),
                kind: RootKind::Symlink { target: "/tool/gcc".to_owned() },
            },
        ),
    ]);
    (tar, expected)
}

#[test]
fn closed_root_file_directory_and_exact_link_pass() {
    let (tar, expected) = fixture();
    assert_eq!(extract_root(deadline(), &tar, &expected, tar.len()), Ok(()));
    assert_eq!(extract_root(deadline(), &tar, &expected, tar.len() - 1), Err(Error::Limit));
    let expired = Instant::now().checked_sub(Duration::from_secs(1)).expect("past");
    assert_eq!(extract_root(expired, &tar, &expected, tar.len()), Err(Error::Deadline));
}

#[test]
fn wrong_link_target_kind_body_mode_and_extra_entries_refuse() {
    let (tar, expected) = fixture();
    for (offset, byte) in [(2048 + 157, b'x'), (2048 + 156, b'1'), (1024 + 106, b'4'), (1536, b'x')]
    {
        let mut bad = tar.clone();
        bad[offset] = byte;
        if offset != 1536 {
            let start = offset / 512 * 512;
            checksum(&mut bad[start..start + 512]);
        }
        assert!(extract_root(deadline(), &bad, &expected, bad.len()).is_err());
    }
    let mut missing = expected.clone();
    missing.remove("/loader");
    assert_eq!(extract_root(deadline(), &tar, &missing, tar.len()), Err(Error::Inventory));
    let mut extra = expected.clone();
    extra.insert(
        "/extra".to_owned(),
        RootEntry { metadata: metadata(0o755), kind: RootKind::Directory },
    );
    assert_eq!(extract_root(deadline(), &tar, &extra, tar.len()), Err(Error::Inventory));
    let mut trailing = tar;
    trailing.extend_from_slice(&[0; 512]);
    assert_eq!(extract_root(deadline(), &trailing, &expected, trailing.len()), Err(Error::Format));
}

#[test]
fn only_exact_absolute_inventory_and_absolute_closed_links_are_allowed() {
    let (tar, expected) = fixture();
    for name in
        ["relative", "/../escape", "/a//b", "/a/./b", "/trailing/", "/space name", "/back\\slash"]
    {
        let mut bad = expected.clone();
        bad.insert(
            name.to_owned(),
            RootEntry { metadata: metadata(0o755), kind: RootKind::Directory },
        );
        assert_eq!(extract_root(deadline(), &tar, &bad, tar.len()), Err(Error::Inventory));
    }
    let mut bad = expected;
    bad.get_mut("/loader").expect("link").kind =
        RootKind::Symlink { target: "../tool/gcc".to_owned() };
    assert_eq!(extract_root(deadline(), &tar, &bad, tar.len()), Err(Error::Inventory));
}

#[test]
fn serialized_expectations_reject_unknown_fields_without_becoming_approval() {
    let (_, expected) = fixture();
    let encoded = serde_json::to_vec(&expected).expect("serialize expected data");
    let decoded: BTreeMap<String, RootEntry> =
        serde_json::from_slice(&encoded).expect("decode expected data");
    assert_eq!(decoded.len(), expected.len());
    assert!(
        serde_json::from_str::<RootEntry>(
            r#"{"metadata":{"uid":0,"gid":0,"mode":365},"kind":"Directory","approved":true}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<RootEntry>(
            r#"{"metadata":{"uid":0,"gid":0,"mode":365,"extra":true},"kind":"Directory"}"#
        )
        .is_err()
    );
}

#[test]
fn root_member_work_bound_has_exact_and_first_extra_cases() {
    let mut tar = record("/", b"", b'5', *b"0000755\0", "");
    let mut expected = BTreeMap::from([(
        String::new(),
        RootEntry { metadata: metadata(0o755), kind: RootKind::Directory },
    )]);
    for index in 0..127 {
        let name = format!("/f{index}");
        tar.extend(record(&name, b"", b'0', *b"0000555\0", ""));
        expected.insert(
            name,
            RootEntry {
                metadata: metadata(0o555),
                kind: RootKind::File { bytes: 0, sha256: Sha256::digest(b"").into() },
            },
        );
    }
    tar.resize(tar.len() + 1024, 0);
    assert_eq!(extract_root(deadline(), &tar, &expected, tar.len()), Ok(()));
    expected.insert(
        "/extra".to_owned(),
        RootEntry { metadata: metadata(0o755), kind: RootKind::Directory },
    );
    assert_eq!(extract_root(deadline(), &tar, &expected, tar.len()), Err(Error::Limit));
}
