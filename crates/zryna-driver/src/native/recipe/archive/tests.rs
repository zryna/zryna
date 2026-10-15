use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::{Error, Expected, FileIdentity, Metadata, Root, extract, header, write_input};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn metadata() -> Metadata {
    Metadata { uid: 0, gid: 0, mode: 0o444 }
}
fn expected(files: &BTreeMap<String, Vec<u8>>) -> Expected {
    Expected {
        root: Root::Input,
        directory: Metadata { mode: 0o555, ..metadata() },
        files: files
            .iter()
            .map(|(name, bytes)| {
                (
                    name.clone(),
                    FileIdentity {
                        metadata: metadata(),
                        bytes: bytes.len(),
                        sha256: Sha256::digest(bytes).into(),
                    },
                )
            })
            .collect(),
    }
}
fn files() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("library.c".to_owned(), b"int value;\n".to_vec()),
        ("sqlite3.h".to_owned(), vec![b'x'; 695_737]),
    ])
}
fn checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}
fn single(header: &[u8; 512], body: &[u8]) -> Vec<u8> {
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(body);
    bytes.resize(512 + super::padded(body.len()).expect("small body") + 1024, 0);
    bytes
}

// Docker GET /archive?path=/input adds the existing directory and prefixes file names.
// Construct that observation separately; PUT targets /input and consumes basenames instead.
fn observed_inventory(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut tar = header::encode("input/", 0, true).expect("GET directory").to_vec();
    for (name, body) in files {
        tar.extend_from_slice(
            &header::encode(&format!("input/{name}"), body.len(), false).expect("GET file"),
        );
        tar.extend_from_slice(body);
        tar.resize(tar.len() + super::padded(body.len()).expect("small fixture") - body.len(), 0);
    }
    tar.resize(tar.len() + 1024, 0);
    tar
}

#[test]
fn observed_input_header_size_accepts_without_package_source_cap() {
    let files = files();
    let expected = expected(&files);
    let tar = observed_inventory(&files);
    assert_eq!(extract(deadline(), &tar, &expected, tar.len()).expect("closed"), files);
    assert_eq!(extract(deadline(), &tar, &expected, tar.len() - 1), Err(Error::Limit));
}

#[test]
fn independent_actual_docker_directory_header_is_accepted() {
    // Literal fields from the retained named-tmpfs Docker USTAR header, not our encoder.
    let mut block = [0; 512];
    block[..6].copy_from_slice(b"input/");
    for (offset, bytes) in [
        (100, b"0000555\0".as_slice()),
        (108, b"0000000\0"),
        (116, b"0000000\0"),
        (124, b"00000000000\0"),
        (136, b"15264155356\0"),
        (148, b"010415\0 "),
        (257, b"ustar\x0000"),
        (329, b"0000000\0"),
        (337, b"0000000\0"),
    ] {
        block[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
    block[156] = b'5';
    let tar = single(&block, b"");
    assert!(
        extract(deadline(), &tar, &expected(&BTreeMap::new()), tar.len())
            .expect("Docker header")
            .is_empty()
    );
}

#[test]
fn output_metadata_and_identity_are_explicit() {
    let body = b"object bytes";
    let mut directory = header::encode("output/", 0, true).expect("directory");
    let mut file = header::encode("output/adapter.o", body.len(), false).expect("file");
    for (block, mode) in [(&mut directory, b"0000700\0"), (&mut file, b"0000644\0")] {
        block[100..108].copy_from_slice(mode);
        block[108..116].copy_from_slice(b"0177776\0");
        block[116..124].copy_from_slice(b"0177776\0");
        checksum(block);
    }
    let mut tar = directory.to_vec();
    tar.extend(single(&file, body));
    let owner = Metadata { uid: 65534, gid: 65534, mode: 0o644 };
    let expected = Expected {
        root: Root::Output,
        directory: Metadata { mode: 0o700, ..owner },
        files: BTreeMap::from([(
            "adapter.o".to_owned(),
            FileIdentity {
                metadata: owner,
                bytes: body.len(),
                sha256: Sha256::digest(body).into(),
            },
        )]),
    };
    assert_eq!(extract(deadline(), &tar, &expected, tar.len()).expect("output")["adapter.o"], body);
}

#[test]
fn all_unsafe_or_nested_input_names_are_refused() {
    for name in ["", ".", "..", "../x", "/x", "nested/x", "a\\b", "a\0b", "é", "space name"] {
        let files = BTreeMap::from([(name.to_owned(), b"x".to_vec())]);
        assert_eq!(write_input(deadline(), &files, 4096), Err(Error::Inventory), "{name:?}");
    }
    let files = BTreeMap::from([("a".repeat(81), vec![])]);
    assert_eq!(write_input(deadline(), &files, 4096), Err(Error::Inventory));
}

#[test]
fn extension_link_prefix_and_nonordinary_numeric_headers_refuse() {
    let files = BTreeMap::from([("a".to_owned(), vec![1])]);
    let tar = observed_inventory(&files);
    for (offset, value) in [
        (156, b'x'),
        (156, b'g'),
        (156, b'L'),
        (156, b'K'),
        (156, b'1'),
        (156, b'2'),
        (156, b'3'),
        (345, b'x'),
        (157, b'x'),
        (108, 0x80),
        (124, b'9'),
        (136, b'-'),
        (265, b'x'),
        (500, b'x'),
    ] {
        let mut bad = tar.clone();
        bad[offset] = value;
        checksum(&mut bad[..512]);
        assert!(
            extract(deadline(), &bad, &expected(&files), bad.len()).is_err(),
            "{offset}:{value}"
        );
    }
    let mut bad = tar.clone();
    bad[257] = b'X';
    checksum(&mut bad[..512]);
    assert_eq!(extract(deadline(), &bad, &expected(&files), bad.len()), Err(Error::Format));
    let mut bad = tar;
    bad[148] ^= 1;
    assert_eq!(extract(deadline(), &bad, &expected(&files), bad.len()), Err(Error::Format));
}

#[test]
fn duplicate_missing_extra_and_trailing_records_refuse() {
    let files = BTreeMap::from([("a".to_owned(), vec![1])]);
    let expected = expected(&files);
    let tar = observed_inventory(&files);
    let mut duplicate = tar[..1536].to_vec();
    duplicate.extend_from_slice(&tar[512..1536]);
    duplicate.extend_from_slice(&[0; 1024]);
    assert_eq!(extract(deadline(), &duplicate, &expected, duplicate.len()), Err(Error::Inventory));
    let mut rootless = tar[512..].to_vec();
    assert_eq!(extract(deadline(), &rootless, &expected, rootless.len()), Err(Error::Inventory));
    rootless.extend_from_slice(&[0; 512]);
    assert!(extract(deadline(), &rootless, &expected, rootless.len()).is_err());
    for extra in [vec![0; 512], vec![1; 512], tar.clone()] {
        let mut bad = tar.clone();
        bad.extend(extra);
        assert_eq!(extract(deadline(), &bad, &expected, bad.len()), Err(Error::Format));
    }
    let extra = BTreeMap::from([("b".to_owned(), vec![1])]);
    assert_eq!(
        extract(deadline(), &tar, &super::tests::expected(&extra), tar.len()),
        Err(Error::Inventory)
    );
    assert!(extract(deadline(), &tar[..tar.len() - 1], &expected, tar.len()).is_err());
}

#[test]
fn wrong_owner_mode_size_body_and_padding_refuse() {
    let files = BTreeMap::from([("a".to_owned(), vec![1])]);
    let expected = expected(&files);
    let tar = observed_inventory(&files);
    for offset in [106, 114, 122, 512 + 106, 512 + 114, 512 + 122, 512 + 134] {
        let mut bad = tar.clone();
        bad[offset] = if bad[offset] == b'1' { b'2' } else { b'1' };
        let start = offset / 512 * 512;
        checksum(&mut bad[start..start + 512]);
        assert_eq!(extract(deadline(), &bad, &expected, bad.len()), Err(Error::Identity));
    }
    let mut bad = tar.clone();
    bad[1024] = 2;
    assert_eq!(extract(deadline(), &bad, &expected, bad.len()), Err(Error::Identity));
    let mut bad = tar;
    bad[1025] = 1;
    assert_eq!(extract(deadline(), &bad, &expected, bad.len()), Err(Error::Format));
}

#[test]
fn deadlines_and_file_count_bounds_have_exact_first_extra() {
    let expired = Instant::now().checked_sub(Duration::from_secs(1)).expect("past");
    let files = BTreeMap::new();
    assert_eq!(write_input(expired, &files, 4096), Err(Error::Deadline));
    assert_eq!(extract(expired, b"", &expected(&files), 4096), Err(Error::Deadline));
    let mut files: BTreeMap<_, _> = (0..64).map(|index| (format!("f{index}"), vec![])).collect();
    write_input(deadline(), &files, 65536).expect("64 upload files");
    let tar = observed_inventory(&files);
    assert_eq!(
        extract(deadline(), &tar, &expected(&files), tar.len()).expect("exact count").len(),
        64
    );
    files.insert("f64".to_owned(), vec![]);
    assert_eq!(write_input(deadline(), &files, 65536), Err(Error::Limit));
    assert_eq!(extract(deadline(), &tar, &expected(&files), tar.len()), Err(Error::Limit));
}
