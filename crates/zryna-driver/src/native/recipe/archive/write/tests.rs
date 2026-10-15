//! PUT archives target an existing volume; GET inventories have a different namespace.
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use sha2::{Digest as _, Sha256};

use super::super::{Error, Expected, FileIdentity, Metadata, Root, extract, padded, write_input};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn upload_contains_only_flat_regular_files_with_exact_root_readonly_metadata() {
    let files = BTreeMap::from([
        ("library.c".to_owned(), b"code".to_vec()),
        ("library.h".to_owned(), b"header".to_vec()),
        ("sqlite3.h".to_owned(), vec![b'x'; 695_737]),
    ]);
    let tar = write_input(deadline(), &files, 2 * 1024 * 1024).expect("bounded upload");
    let mut offset = 0;
    for (name, body) in &files {
        let block = &tar[offset..offset + 512];
        let end = block[..100].iter().position(|byte| *byte == 0).expect("terminated name");
        assert_eq!(&block[..end], name.as_bytes());
        assert!(!block[..end].contains(&b'/'));
        assert_eq!(block[156], b'0');
        assert_eq!(&block[100..108], b"0000444\0");
        assert_eq!(&block[108..116], b"0000000\0");
        assert_eq!(&block[116..124], b"0000000\0");
        assert_eq!(&block[257..265], b"ustar\x0000");
        assert_eq!(&tar[offset + 512..offset + 512 + body.len()], body);
        offset += 512 + padded(body.len()).expect("bounded body");
    }
    assert_eq!(tar.len(), offset + 1024);
    assert!(tar[offset..].iter().all(|byte| *byte == 0));
}

#[test]
fn upload_exact_byte_limit_and_empty_archive_omit_directory_overhead() {
    let empty = write_input(deadline(), &BTreeMap::new(), 1024).expect("empty upload");
    assert_eq!(empty, vec![0; 1024]);
    assert_eq!(write_input(deadline(), &BTreeMap::new(), 1023), Err(Error::Limit));
    let files = BTreeMap::from([("a".to_owned(), vec![7; 513])]);
    let exact = 512 + 1024 + 1024;
    assert_eq!(write_input(deadline(), &files, exact).expect("exact").len(), exact);
    assert_eq!(write_input(deadline(), &files, exact - 1), Err(Error::Limit));
}

#[test]
fn upload_namespace_cannot_masquerade_as_complete_get_inventory() {
    let files = BTreeMap::from([("library.c".to_owned(), b"code".to_vec())]);
    let upload = write_input(deadline(), &files, 4096).expect("flat upload");
    let metadata = Metadata { uid: 0, gid: 0, mode: 0o444 };
    let expected = Expected {
        root: Root::Input,
        directory: Metadata { mode: 0o555, ..metadata },
        files: BTreeMap::from([(
            "library.c".to_owned(),
            FileIdentity { metadata, bytes: 4, sha256: Sha256::digest(b"code").into() },
        )]),
    };
    assert_eq!(extract(deadline(), &upload, &expected, upload.len()), Err(Error::Inventory));
}
