//! Genuine current hosted ELF/disassembly fixtures are parsed as data, never executed.
use super::super::super::{machine, targets};
use super::*;
fn decoded_fixture(text: &str, expected_bytes: usize, expected_sha256: &str) -> Vec<u8> {
    let text = text.strip_suffix('\n').expect("canonical hex fixture newline");
    assert_eq!(text.len(), expected_bytes * 2);
    assert!(text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    let bytes = text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex pair"), 16)
                .expect("two hexadecimal digits")
        })
        .collect::<Vec<_>>();
    assert_eq!(bytes.len(), expected_bytes);
    assert_eq!(hex(&sha(&bytes)), expected_sha256);
    bytes
}
fn fixtures() -> [(&'static Bindings, Vec<u8>, &'static [u8]); 3] {
    [
        (
            &positive::VALUE,
            decoded_fixture(
                include_str!("fixtures/positive.elf.hex"),
                122_024,
                "c285fd25ce134e9daf5fe76404c61190ae56b55a23f742df3fedbcbd0906d487",
            ),
            include_bytes!("fixtures/positive.disassembly"),
        ),
        (
            &heap::VALUE,
            decoded_fixture(
                include_str!("fixtures/heap.elf.hex"),
                122_976,
                "fc03b30995c4eab72cbaed4636767c4fe73aafe266fb0b649ccbf37882908c7b",
            ),
            include_bytes!("fixtures/heap.disassembly"),
        ),
        (
            &signed::VALUE,
            decoded_fixture(
                include_str!("fixtures/signed.elf.hex"),
                122_480,
                "985de55740a0ea2924ce07a2c518d5b74aafea81c2b6c0fcd6c0ee9e9693c598",
            ),
            include_bytes!("fixtures/signed.disassembly"),
        ),
    ]
}
#[test]
fn owned_profile_matches_all_three_current_full_vectors_and_actual_call_targets() {
    for (bindings, elf, disassembly) in fixtures() {
        assert_eq!(bindings.mnemonics.len(), 343);
        assert!(targets::compare(&elf, disassembly, bindings).is_ok());
    }
    assert_eq!(positive::VALUE.undefined_symbols.len(), 32);
    assert_eq!(heap::VALUE.undefined_symbols.len(), 35);
    assert_eq!(signed::VALUE.undefined_symbols.len(), 34);
}
#[test]
fn owned_profile_rejects_missing_extra_or_changed_machine_vectors() {
    for (bindings, elf, _) in fixtures() {
        assert!(bindings.layout(&elf, &bindings.mnemonics[..342]).is_err());
        let mut extra = bindings.mnemonics.to_vec();
        extra.push("nop");
        assert!(bindings.layout(&elf, &extra).is_err());
        extra[0] = "ret";
        extra.pop();
        assert!(bindings.layout(&elf, &extra).is_err());
        let mut changed = elf.clone();
        let mut names = 0;
        for at in 0..changed.len().saturating_sub(5) {
            if &changed[at..at + 5] == b"puts\0" {
                changed[at] = b'x';
                names += 1;
            }
        }
        assert!(names > 0);
        assert!(bindings.layout(&changed, bindings.mnemonics).is_err());
    }
}
#[test]
fn owned_profile_rejects_detached_actual_caller_redirect_without_a_target_run() {
    for (bindings, elf, disassembly) in fixtures() {
        let call = targets::compare(&elf, disassembly, bindings).expect("original forward");
        let changed = machine::redirect(&elf, call).expect("memory-only caller control");
        let differences = elf
            .iter()
            .zip(&changed)
            .enumerate()
            .filter_map(|(at, (a, b))| (a != b).then_some(at))
            .collect::<Vec<_>>();
        assert!(!differences.is_empty() && differences.len() <= 4);
        assert!(differences.last().expect("changed byte") - differences[0] < 4);
        assert!(targets::compare(&changed, disassembly, bindings).is_err());
        assert!(targets::compare(&elf, disassembly, bindings).is_ok());
    }
}
#[test]
fn owned_profile_rejects_unsupported_elf_target_before_layout_authority() {
    for (bindings, elf, _) in fixtures() {
        let mut changed = elf.clone();
        changed[18] = 0xb7;
        changed[19] = 0;
        assert!(bindings.layout(&changed, bindings.mnemonics).is_err());
    }
}
#[test]
fn owned_profile_rejects_wrong_case_or_empty_material_inventory_before_any_path_read() {
    assert!(positive::VALUE.materials(Case::HeapOverrun).is_err());
    let missing = Bindings {
        case: Case::Positive,
        hooks: positive::VALUE.hooks,
        mnemonics: positive::VALUE.mnemonics,
        undefined_symbols: positive::VALUE.undefined_symbols,
        providers: &[],
    };
    assert!(missing.materials(Case::Positive).is_err());
}
#[test]
fn owned_profile_rejects_unknown_exact_tools_missing_extra_and_replacement_metadata() {
    let profile = &hosted::VALUE;
    let pins = profile
        .tools
        .iter()
        .map(|pin| {
            serde_json::json!({
                "path":pin.path,"bytes":pin.bytes,"sha256":hex(&pin.sha256)
            })
        })
        .collect::<Vec<_>>();
    assert!(profile.check_pins(&pins).is_ok()); // No RuntimeProfile constructed from JSON.
    assert!(profile.check_pins(&pins[..2]).is_err());
    let mut extra = pins.clone();
    extra.push(pins[0].clone());
    assert!(profile.check_pins(&extra).is_err());
    for (key, value) in [
        ("path", serde_json::json!("/usr/bin/x86_64-linux-gnu-gcc-14")),
        ("bytes", serde_json::json!(1_023_033)),
        (
            "sha256",
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000"),
        ),
    ] {
        let mut changed = pins.clone();
        changed[0][key] = value;
        assert!(profile.check_pins(&changed).is_err());
    }
}
#[test]
fn owning_runtime_profile_does_not_change_default_compiler_only_none_policy() {
    for case in [Case::Positive, Case::HeapOverrun, Case::SignedOverflow] {
        assert!(reviewed(case).is_none());
    }
}
