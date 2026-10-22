use super::*;

fn fixture(entries: &[(&str, &[u8], u32, u32)]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(
        &archive_header::encode("output/", 0, true, 65534, 65534, 0o700).expect("directory"),
    );
    for (name, body, uid, mode) in entries {
        data.extend_from_slice(
            &archive_header::encode(name, body.len(), false, *uid, *uid, *mode).expect("member"),
        );
        data.extend_from_slice(body);
        data.resize(data.len().div_ceil(512) * 512, 0);
    }
    data.resize(data.len() + 1024, 0);
    data
}

#[test]
fn actual_output_inventory_rejects_extra_executable_missing_alias_or_duplicate_members() {
    let good =
        fixture(&[("output/stdout", b"frame", 65534, 0o600), ("output/stderr", b"", 65534, 0o600)]);
    assert_eq!(archive::output(&good, Operation::Compile, future()).expect("output").0, b"frame");
    for entries in [
        vec![("output/stdout", b"frame".as_slice(), 65534, 0o600)],
        vec![
            ("output/stdout", b"frame".as_slice(), 65534, 0o600),
            ("output/stderr", b"", 65534, 0o600),
            ("output/extra.elf", b"ELF", 65534, 0o755),
        ],
        vec![
            ("output/stdout", b"frame".as_slice(), 65534, 0o600),
            ("output/stdout", b"", 65534, 0o600),
        ],
        vec![
            ("output/../stdout", b"frame".as_slice(), 65534, 0o600),
            ("output/stderr", b"", 65534, 0o600),
        ],
        vec![
            ("output/stdout", b"frame".as_slice(), 0, 0o600),
            ("output/stderr", b"", 65534, 0o600),
        ],
        vec![
            ("output/stdout", b"frame".as_slice(), 65534, 0o755),
            ("output/stderr", b"", 65534, 0o600),
        ],
    ] {
        assert!(archive::output(&fixture(&entries), Operation::Compile, future()).is_err());
    }
    for kind in *b"12xg" {
        let mut alias = good.clone();
        alias[512 + 156] = kind;
        repair_checksum(&mut alias[512..1024]);
        assert!(archive::output(&alias, Operation::Compile, future()).is_err());
    }
    let mut fragment = good.clone();
    fragment.push(0);
    assert!(archive::output(&fragment, Operation::Compile, future()).is_err());
}

fn repair_checksum(header: &mut [u8]) {
    header[148..156].fill(b' ');
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
}

#[test]
fn compiler_and_evaluator_outputs_have_independent_exact_first_extra_counters() {
    for (op, limit) in [(Operation::Compile, 1_277_956), (Operation::Evaluate, 16_384)] {
        let stdout = vec![b'x'; limit];
        let stderr = vec![b'e'; 4096];
        let exact = fixture(&[
            ("output/stdout", &stdout, 65534, 0o600),
            ("output/stderr", &stderr, 65534, 0o600),
        ]);
        assert!(archive::output(&exact, op, future()).is_ok());
        let extra = vec![b'x'; limit + 1];
        assert!(
            archive::output(
                &fixture(&[
                    ("output/stdout", &extra, 65534, 0o600),
                    ("output/stderr", &[], 65534, 0o600)
                ]),
                op,
                future()
            )
            .is_err()
        );
        let extra = vec![b'e'; 4097];
        assert!(
            archive::output(
                &fixture(&[
                    ("output/stdout", &[], 65534, 0o600),
                    ("output/stderr", &extra, 65534, 0o600)
                ]),
                op,
                future()
            )
            .is_err()
        );
    }
}

#[test]
fn input_capture_is_one_root_owned_readonly_request_and_deadlines_are_not_renewed() {
    let bytes = archive::input(b"source", future()).expect("input");
    let header = archive_header::parse(&bytes[..512]).expect("header");
    assert_eq!(header.name, "request");
    assert_eq!((header.uid, header.gid, header.mode), (0, 0, 0o444));
    assert!(archive::input(b"source", Instant::now()).is_err());
    let mut corruption = bytes.clone();
    corruption[0] = b'x';
    assert!(archive_header::parse(&corruption[..512]).is_err());
}
