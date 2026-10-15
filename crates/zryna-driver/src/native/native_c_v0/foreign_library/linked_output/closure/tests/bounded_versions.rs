//! Independently encoded repeated VERDEF names exercise decoded, not raw-file, bounds.
use super::*;

fn repeated_versions(name: &[u8], groups: &[(Vec<u8>, usize)]) -> Vec<u8> {
    let mut bytes = minimal(3, name);
    bytes.resize(17_000, 0);
    write(&mut bytes, 64 + 32, 17_000, 8);
    write(&mut bytes, 64 + 40, 17_000, 8);
    write(&mut bytes, 40, 16_384, 8);
    write(&mut bytes, 60, 7, 2);
    // Construct an independent string table with one unique name per definition.
    let mut strings = vec![0];
    let mut offsets = Vec::new();
    for (name, _) in groups {
        offsets.push(strings.len());
        strings.extend_from_slice(name);
        strings.push(0);
    }
    let soname = strings.len();
    strings.extend_from_slice(name);
    strings.push(0);
    bytes[512..512 + strings.len()].copy_from_slice(&strings);
    let entries = [
        (5, 512),
        (10, strings.len() as u64),
        (6, 12_800),
        (11, 24),
        (4, 12_832),
        (14, soname as u64),
        (0x6fff_fffc, 12_900),
        (0x6fff_fffd, groups.len() as u64),
        (0x6fff_fff0, 16_000),
        (0, 0),
    ];
    for (i, (tag, value)) in entries.iter().enumerate() {
        write(&mut bytes, 256 + i * 16, *tag, 8);
        write(&mut bytes, 264 + i * 16, *value, 8);
    }
    write(&mut bytes, 152, 160, 8);
    write(&mut bytes, 160, 160, 8);
    bytes[12_800..12_824].fill(0);
    write(&mut bytes, 12_832, 1, 4);
    write(&mut bytes, 12_836, 1, 4);
    let mut position = 12_900;
    for (i, (name, count)) in groups.iter().enumerate() {
        let size = 20 + count * 8;
        for (field, value, width) in [
            (0, 1, 2),
            (4, (i + 1) as u64, 2),
            (6, *count as u64, 2),
            (8, u64::from(symbols::sysv_hash(name)), 4),
            (12, 20, 4),
            (16, if i + 1 == groups.len() { 0 } else { size as u64 }, 4),
        ] {
            write(&mut bytes, position + field, value, width);
        }
        for aux in 0..*count {
            write(&mut bytes, position + 20 + aux * 8, offsets[i] as u64, 4);
            write(&mut bytes, position + 24 + aux * 8, if aux + 1 == *count { 0 } else { 8 }, 4);
        }
        position += size;
    }
    // Encode an independent section table after the long strings, with exact load-backed offsets.
    bytes[16_384..16_384 + 7 * 64].fill(0);
    for (i, kind, off, size, link, entry) in [
        (1, 3, 512_usize, strings.len(), 0, 0),
        (2, 11, 12_800, 24, 1, 24),
        (3, 5, 12_832, 16, 2, 4),
        (5, 0x6fff_fffd, 12_900, position - 12_900, 1, 0),
        (6, 0x6fff_ffff, 16_000, 2, 2, 2),
    ] {
        let row = 16_384 + i * 64;
        for (field, value, width) in [
            (4, kind, 4),
            (16, off as u64, 8),
            (24, off as u64, 8),
            (32, size as u64, 8),
            (40, link, 4),
            (56, entry, 8),
        ] {
            write(&mut bytes, row + field, value, width);
        }
    }
    write(&mut bytes, 16_384 + 5 * 64 + 44, groups.len() as u64, 4);
    bytes
}

#[test]
fn repeated_version_names_exact_limit_and_first_extra_are_bounded_before_decode() {
    let mut groups = vec![(vec![b'A'; 4096], 128), (vec![b'B'; 4096], 128)];
    let exact = repeated_versions(b"provider.so", &groups);
    assert_eq!(
        inspect(&exact, 3, &policy()).expect("exact decoded name budget").name_bytes(),
        image::MAX_STRINGS
    );
    groups.push((vec![b'C'], 1));
    assert!(inspect(&repeated_versions(b"provider.so", &groups), 3, &policy()).is_err());
}

#[test]
fn repeated_version_names_graph_exact_limit_and_first_extra_are_bounded() {
    let provider =
        repeated_versions(b"provider.so", &[(vec![b'A'; 4096], 128), (vec![b'B'; 4096], 128)]);
    let loader = minimal(3, b"loader.so");
    let mut expected = policy();
    expected.needed = vec!["provider.so".into()];
    let materials = vec![
        RetainedProvider { id: "provider".into(), bytes: provider.into() },
        RetainedProvider { id: "loader".into(), bytes: loader.into() },
    ];
    expected.providers = materials
        .iter()
        .zip(["provider.so", "loader.so"])
        .map(|(m, name)| ProviderRule {
            id: m.id.clone(),
            sha256: Sha256::digest(&m.bytes).into(),
            soname: Some(name.into()),
            needed: vec![],
        })
        .collect();
    assert!(validate_providers(&materials, &expected).is_ok());
    let mut extra = materials;
    extra[1].bytes = repeated_versions(b"loader.so", &[(vec![b'C'], 1)]).into();
    expected.providers[1].sha256 = Sha256::digest(&extra[1].bytes).into();
    assert!(validate_providers(&extra, &expected).is_err());
}
