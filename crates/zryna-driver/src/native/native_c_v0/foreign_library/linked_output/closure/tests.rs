//! Independently encoded ELF records, not serializer-generated parser mirrors or approvals.
use super::*;
#[path = "tests/bounded_versions.rs"]
mod bounded_versions;
#[path = "tests/hostile_symbols.rs"]
mod hostile_symbols;

fn write(bytes: &mut [u8], offset: usize, value: u64, width: usize) {
    bytes[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
}
fn section(
    bytes: &mut [u8],
    index: usize,
    kind: u64,
    offset: u64,
    size: u64,
    link: u64,
    entry: u64,
) {
    let row = 1024 + index * 64;
    for (field, value, width) in [
        (4, kind, 4),
        (16, offset, 8),
        (24, offset, 8),
        (32, size, 8),
        (40, link, 4),
        (56, entry, 8),
    ] {
        write(bytes, row + field, value, width);
    }
}
fn minimal(kind: u16, name: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0_u8; 1408];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    for (offset, value, width) in [
        (16, u64::from(kind), 2),
        (18, 62, 2),
        (20, 1, 4),
        (32, 64, 8),
        (40, 1024, 8),
        (52, 64, 2),
        (54, 56, 2),
        (56, 2, 2),
        (58, 64, 2),
        (60, 5, 2),
    ] {
        write(&mut bytes, offset, value, width);
    }
    for (offset, value, width) in [
        (64, 1, 4),
        (64 + 32, 1408, 8),
        (64 + 40, 1408, 8),
        (120, 2, 4),
        (128, 256, 8),
        (136, 256, 8),
        (152, 112, 8),
        (160, 112, 8),
    ] {
        write(&mut bytes, offset, value, width);
    }
    let entries = [(5, 512), (10, 64), (6, 640), (11, 24), (4, 768), (14, 1), (0, 0)];
    for (index, (tag, value)) in entries.iter().enumerate() {
        write(&mut bytes, 256 + index * 16, *tag, 8);
        write(&mut bytes, 264 + index * 16, *value, 8);
    }
    bytes[513..513 + name.len()].copy_from_slice(name);
    section(&mut bytes, 1, 3, 512, 64, 0, 0);
    section(&mut bytes, 2, 11, 640, 24, 1, 24);
    section(&mut bytes, 3, 5, 768, 16, 2, 4);
    write(&mut bytes, 768, 1, 4);
    write(&mut bytes, 772, 1, 4);
    bytes
}
fn policy() -> ProviderExpectation {
    ProviderExpectation {
        interpreter: "/lib/loader.so".into(),
        needed: vec![],
        providers: vec![],
        allow_weak: false,
        allow_ifunc: false,
        allow_tls: false,
    }
}

#[test]
fn independently_encoded_dynamic_object_and_closed_hostile_records() {
    let bytes = minimal(3, b"provider.so");
    let observed = inspect(&bytes, 3, &policy()).expect("independent ELF fixture");
    assert_eq!(observed.soname.as_deref(), Some("provider.so"));
    assert_eq!(observed.symbols.len(), 1);
    let mut linux = bytes.clone();
    linux[7] = 3;
    assert!(inspect(&linux, 3, &policy()).is_ok());
    for (offset, value, width) in [
        (7, 9, 1),
        (8, 1, 1),
        (9, 1, 1),
        (15, 1, 1),
        (18, 3, 2),
        (32, u64::MAX, 8),
        (152, 113, 8),
        (256, 29, 8),
        (264, 1409, 8),
        (280, 2_097_152, 8),
        (312, 16, 8),
        (344, 63, 8),
        (352, 1, 8),
        (768, 0, 4),
        (772, 16_385, 4),
        (780, 1, 4),
        (1024 + 2 * 64 + 40, 2, 4),
    ] {
        let mut mutant = bytes.clone();
        write(&mut mutant, offset, value, width);
        assert!(inspect(&mutant, 3, &policy()).is_err(), "mutation {offset}");
    }
    for cut in [0, 6, 63, 119, 255, 767, 1023, 1407] {
        assert!(inspect(&bytes[..cut], 3, &policy()).is_err());
    }
}

#[test]
fn dynamic_effects_and_ambiguous_active_mapping_are_rejected() {
    let bytes = minimal(3, b"provider.so");
    for tag in [15, 16, 22, 29, 0x6fff_fefb, 0x7fff_ffff, 0x6fff_fefc] {
        let mut mutant = bytes.clone();
        write(&mut mutant, 336, tag, 8);
        assert!(inspect(&mutant, 3, &policy()).is_err());
    }
    let mut overlap = bytes.clone();
    write(&mut overlap, 120, 1, 4);
    assert!(inspect(&overlap, 3, &policy()).is_err());
}

#[test]
fn load_bss_overlap_and_equal_byte_section_offset_alias_do_not_authorize_mapping() {
    let bytes = minimal(3, b"provider.so");
    let mut overlap = bytes.clone();
    write(&mut overlap, 56, 3, 2);
    for (offset, value, width) in
        [(176, 1, 4), (184, 900, 8), (192, 900, 8), (208, 0, 8), (216, 800, 8)]
    {
        write(&mut overlap, offset, value, width);
    }
    assert!(inspect(&overlap, 3, &policy()).is_err());
    let mut alias = bytes.clone();
    alias.copy_within(512..576, 896);
    write(&mut alias, 1024 + 64 + 24, 896, 8);
    assert!(inspect(&alias, 3, &policy()).is_err());
    let mut dynamic = bytes.clone();
    write(&mut dynamic, 136, 384, 8);
    assert!(inspect(&dynamic, 3, &policy()).is_err());
    let mut overflow = bytes.clone();
    write(&mut overflow, 80, u64::MAX - 10, 8);
    assert!(inspect(&overflow, 3, &policy()).is_err());
}

#[test]
fn independent_version_records_reject_overlap_truncation_wrong_links_and_counts() {
    let mut bytes = minimal(3, b"provider.so");
    // Replace the seven-entry dynamic list with ten entries and add VERDEF/VERSYM sections.
    write(&mut bytes, 152, 160, 8);
    write(&mut bytes, 160, 160, 8);
    write(&mut bytes, 60, 6, 2);
    for (index, (tag, value)) in
        [(0x6fff_fffc, 832), (0x6fff_fffd, 1), (0x6fff_fff0, 896), (0, 0)].iter().enumerate()
    {
        write(&mut bytes, 352 + index * 16, *tag, 8);
        write(&mut bytes, 360 + index * 16, *value, 8);
    }
    section(&mut bytes, 4, 0x6fff_fffd, 832, 28, 1, 0);
    write(&mut bytes, 1024 + 4 * 64 + 44, 1, 4);
    section(&mut bytes, 5, 0x6fff_ffff, 896, 2, 2, 2);
    for (offset, value, width) in
        [(832, 1, 2), (834, 1, 2), (836, 1, 2), (838, 1, 2), (844, 20, 4), (852, 1, 4)]
    {
        write(&mut bytes, offset, value, width);
    }
    write(&mut bytes, 840, u64::from(symbols::sysv_hash(b"provider.so")), 4);
    assert!(inspect(&bytes, 3, &policy()).is_ok());
    for (offset, value, width) in [
        (832, 2, 2),
        (838, 0, 2),
        (844, 4, 4),
        (848, 20, 4),
        (856, 8, 4),
        (896, 3, 2),
        (1024 + 4 * 64 + 40, 2, 4),
        (1024 + 4 * 64 + 44, 2, 4),
        (1024 + 5 * 64 + 32, 4, 8),
    ] {
        let mut mutant = bytes.clone();
        write(&mut mutant, offset, value, width);
        assert!(inspect(&mutant, 3, &policy()).is_err(), "version mutation {offset}");
    }
}

#[test]
fn explicit_provider_policy_and_identity_are_closed_data() {
    let bytes = minimal(3, b"provider.so");
    let provider = RetainedProvider { id: "retainedLibrary".into(), bytes: bytes.clone().into() };
    let rule = ProviderRule {
        id: "retainedLibrary".into(),
        sha256: Sha256::digest(&bytes).into(),
        soname: Some("provider.so".into()),
        needed: vec![],
    };
    let mut expected = policy();
    expected.providers.push(rule);
    assert!(audit(&bytes, &[provider], &expected).is_err()); // ET_DYN cannot become the executable.
    assert!(serde_json::from_str::<ProviderExpectation>(r#"{"interpreter":"/lib/a","needed":[],"providers":[],"allow_weak":false,"allow_ifunc":false,"allow_tls":false,"execution_authorized":true}"#).is_err());
}

fn executable() -> Vec<u8> {
    let mut bytes = minimal(2, b"provider.so");
    write(&mut bytes, 56, 3, 2);
    for (offset, value, width) in
        [(176, 3, 4), (184, 448, 8), (192, 448, 8), (208, 17, 8), (216, 17, 8), (336, 1, 8)]
    {
        write(&mut bytes, offset, value, width);
    }
    bytes[448..465].copy_from_slice(b"/lib/provider.so\0");
    bytes
}
#[test]
fn complete_independent_graph_has_no_authority_and_rejects_missing_extra_ordered_inputs() {
    let provider = minimal(3, b"provider.so");
    let mut expected = policy();
    expected.interpreter = "/lib/provider.so".into();
    expected.needed = vec!["provider.so".into()];
    expected.providers = vec![ProviderRule {
        id: "retainedLibrary".into(),
        sha256: Sha256::digest(&provider).into(),
        soname: Some("provider.so".into()),
        needed: vec![],
    }];
    let materials = vec![RetainedProvider { id: "retainedLibrary".into(), bytes: provider.into() }];
    let result = audit(&executable(), &materials, &expected).expect("independent encoded graph");
    assert!(
        !result.execution_authorized
            && !result.complete_input_use_attestation
            && !result.relocation_or_resolver_execution_attested
    );
    assert_eq!(result.providers.len(), 1);
    assert!(audit(&executable(), &[], &expected).is_err());
    let mut changed = materials.clone();
    changed.push(changed[0].clone());
    assert!(audit(&executable(), &changed, &expected).is_err());
    changed.truncate(1);
    let mut replaced = changed[0].bytes.to_vec();
    replaced[600] = 1;
    changed[0].bytes = replaced.into();
    assert!(audit(&executable(), &changed, &expected).is_err());
    let mut wrong = expected.clone();
    wrong.needed.clear();
    assert!(audit(&executable(), &materials, &wrong).is_err());
    wrong = expected.clone();
    wrong.providers[0].needed.push("missing.so".into());
    assert!(audit(&executable(), &materials, &wrong).is_err());
    let mut duplicate = materials.clone();
    let mut other = duplicate[0].bytes.to_vec();
    other[900] = 1;
    duplicate.push(RetainedProvider { id: "otherLibrary".into(), bytes: other.into() });
    let mut alias = expected.clone();
    let mut rule = alias.providers[0].clone();
    rule.id = "otherLibrary".into();
    rule.sha256 = Sha256::digest(&duplicate[1].bytes).into();
    alias.providers.push(rule);
    assert!(audit(&executable(), &duplicate, &alias).is_err());
}
