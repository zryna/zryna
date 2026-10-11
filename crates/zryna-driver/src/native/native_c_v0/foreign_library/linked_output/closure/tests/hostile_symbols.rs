//! Real symbol requirements, explicit exceptional kinds and independently malformed hashes.
use super::*;

fn with_symbol(mut bytes: Vec<u8>, defined: bool, kind: u8) -> Vec<u8> {
    bytes[525..531].copy_from_slice(b"entry\0");
    write(&mut bytes, 1024 + 2 * 64 + 32, 48, 8);
    write(&mut bytes, 664, 13, 4);
    bytes[668] = 0x10 | kind;
    write(&mut bytes, 670, u64::from(defined), 2);
    write(&mut bytes, 1024 + 3 * 64 + 32, 20, 8);
    write(&mut bytes, 772, 2, 4);
    write(&mut bytes, 776, 1, 4);
    bytes
}
#[test]
fn strong_symbol_resolution_and_explicit_weak_ifunc_tls_data() {
    let input = with_symbol(executable(), false, 2);
    let provider = with_symbol(minimal(3, b"provider.so"), true, 2);
    let mut expected = policy();
    expected.interpreter = "/lib/provider.so".into();
    expected.needed = vec!["provider.so".into()];
    expected.providers = vec![ProviderRule {
        id: "material".into(),
        sha256: Sha256::digest(&provider).into(),
        soname: Some("provider.so".into()),
        needed: vec![],
    }];
    let materials = vec![RetainedProvider { id: "material".into(), bytes: provider.into() }];
    let observed = audit(&input, &materials, &expected)
        .expect("independently encoded strong import and provider");
    assert_eq!(observed.executable.symbols[1].name, "entry");
    let absent = with_symbol(minimal(3, b"provider.so"), false, 2);
    let mut absent_expected = expected.clone();
    absent_expected.providers[0].sha256 = Sha256::digest(&absent).into();
    assert!(
        audit(
            &input,
            &[RetainedProvider { id: "material".into(), bytes: absent.into() }],
            &absent_expected
        )
        .is_err()
    );
    for kind in [6, 10] {
        let bytes = with_symbol(minimal(3, b"provider.so"), true, kind);
        assert!(inspect(&bytes, 3, &policy()).is_err());
        let mut allow = policy();
        allow.allow_tls = true;
        allow.allow_ifunc = true;
        assert_eq!(inspect(&bytes, 3, &allow).expect("explicit kind data").symbols[1].kind, kind);
    }
    let mut weak = input;
    weak[668] = 0x22;
    assert!(inspect(&weak, 2, &expected).is_err());
    expected.allow_weak = true;
    assert_eq!(inspect(&weak, 2, &expected).expect("explicit weak data").symbols[1].binding, 2);
    assert!(audit(&weak, &materials, &expected).is_ok());
    for binding in [1, 2] {
        for visibility in [1, 2, 3] {
            let mut mutant = weak.clone();
            mutant[668] = (binding << 4) | 2;
            mutant[669] = visibility;
            assert!(audit(&mutant, &materials, &expected).is_err());
        }
    }
}

#[test]
fn sysv_hash_cycles_counts_and_names_are_not_trusted() {
    let bytes = with_symbol(minimal(3, b"provider.so"), true, 2);
    assert!(inspect(&bytes, 3, &policy()).is_ok());
    for (offset, value) in [(768, 0), (772, 3), (776, 2), (784, 1), (664, 63)] {
        let mut mutant = bytes.clone();
        write(&mut mutant, offset, value, 4);
        assert!(inspect(&mutant, 3, &policy()).is_err());
    }
    let mut common = bytes;
    write(&mut common, 670, 0xfff2, 2);
    assert!(inspect(&common, 3, &policy()).is_err());
}

#[test]
fn independently_encoded_empty_gnu_lookup_retains_imports_but_never_exports() {
    let mut bytes = with_symbol(minimal(3, b"provider.so"), false, 2);
    write(&mut bytes, 320, 0x6fff_fef5, 8);
    write(&mut bytes, 1024 + 3 * 64 + 4, 0x6fff_fff6, 4);
    write(&mut bytes, 1024 + 3 * 64 + 32, 28, 8);
    write(&mut bytes, 1024 + 3 * 64 + 56, 0, 8);
    bytes[768..796].fill(0);
    for offset in [768, 772, 776] {
        write(&mut bytes, offset, 1, 4);
    }
    let mut expected = policy();
    expected.allow_weak = true;
    for binding in [1, 2] {
        bytes[668] = (binding << 4) | 2;
        let observation = inspect(&bytes, 3, &expected).expect("valid import-only GNU lookup");
        assert!(!observation.symbols[1].defined);
        assert!(!observation.symbols[1].lookup_reachable);
    }
    for (offset, value, width) in [(670, 1, 2), (784, 1, 8), (792, 1, 4)] {
        let mut mutant = bytes.clone();
        write(&mut mutant, offset, value, width);
        assert!(inspect(&mutant, 3, &expected).is_err());
    }
}

#[test]
fn relative_relocation_counts_require_present_bounded_tables() {
    for (address, size, entry, count, width) in
        [(7, 8, 9, 0x6fff_fff9, 24), (17, 18, 19, 0x6fff_fffa, 16)]
    {
        let mut missing = minimal(3, b"provider.so");
        write(&mut missing, 336, count, 8);
        assert!(inspect(&missing, 3, &policy()).is_err());
        let mut valid = minimal(3, b"provider.so");
        write(&mut valid, 152, 176, 8);
        write(&mut valid, 160, 176, 8);
        for (index, (tag, value)) in
            [(address, 896), (size, width), (entry, width), (count, 1), (0, 0)].iter().enumerate()
        {
            write(&mut valid, 352 + index * 16, *tag, 8);
            write(&mut valid, 360 + index * 16, *value, 8);
        }
        assert!(inspect(&valid, 3, &policy()).is_ok());
        write(&mut valid, 408, 2, 8);
        assert!(inspect(&valid, 3, &policy()).is_err());
    }
}

#[test]
fn nonempty_gnu_hash_import_prefix_cannot_hide_lookup_eligible_definitions() {
    let mut bytes = with_symbol(minimal(3, b"provider.so"), false, 2);
    bytes[531..537].copy_from_slice(b"other\0");
    write(&mut bytes, 1024 + 2 * 64 + 32, 72, 8);
    write(&mut bytes, 688, 19, 4);
    bytes[692] = 0x12;
    write(&mut bytes, 694, 1, 2);
    write(&mut bytes, 320, 0x6fff_fef5, 8);
    write(&mut bytes, 1024 + 3 * 64 + 4, 0x6fff_fff6, 4);
    write(&mut bytes, 1024 + 3 * 64 + 32, 32, 8);
    write(&mut bytes, 1024 + 3 * 64 + 56, 0, 8);
    bytes[768..800].fill(0);
    let hash = symbols::gnu_hash(b"other");
    for (offset, value, width) in [
        (768, 1, 4),
        (772, 2, 4),
        (776, 1, 4),
        (784, 1_u64 << (hash % 64), 8),
        (792, 2, 4),
        (796, u64::from(hash | 1), 4),
    ] {
        write(&mut bytes, offset, value, width);
    }
    let observed = inspect(&bytes, 3, &policy()).expect("undefined import prefix is valid");
    assert!(!observed.symbols[1].lookup_reachable);
    assert!(observed.symbols[2].lookup_reachable);
    for visibility in [0, 3] {
        let mut mutant = bytes.clone();
        write(&mut mutant, 670, 1, 2);
        mutant[669] = visibility;
        assert!(inspect(&mutant, 3, &policy()).is_err());
    }
}
