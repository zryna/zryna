//! Independent ELF mutations retain the full original audit suite and exact release edge checks.
use object::{Object, ObjectSection, ObjectSymbol};
use zryna_native_mir::generic_owned_v2::VerifiedProgram;

pub(super) fn hostile_release_inventory(bytes: &[u8], mir: &VerifiedProgram<'_, '_>) {
    super::hostile::hostile_inventory(bytes, mir, b"zryna_v1_e_none");
    let file = object::File::parse(bytes).expect("independent ELF reader");
    let declaration = |operation| {
        mir.program()
            .runtime()
            .declarations()
            .native_linux_x86_64
            .iter()
            .find(|entry| entry.operation == operation)
            .expect("fixed runtime operation")
            .symbol
            .as_str()
    };
    let symbol = |name| {
        file.symbols()
            .find(|entry| entry.name().ok() == Some(name))
            .expect("exact retained runtime symbol")
            .index()
            .0
    };
    let release = symbol(declaration("stringRelease"));
    let clone = symbol(declaration("stringClone"));
    let (offset, size) = file
        .section_by_name(".rela.text")
        .expect("runtime call relocations")
        .file_range()
        .expect("stored ELF relocation section");
    let begin = usize::try_from(offset).expect("file offset");
    let size = usize::try_from(size).expect("file extent");
    assert_eq!(size % 24, 0, "closed ELF64 RELA shape");
    let record = (begin..begin + size)
        .step_by(24)
        .find(|record| {
            let info =
                u64::from_le_bytes(bytes[record + 8..record + 16].try_into().expect("ELF64 info"));
            usize::try_from(info >> 32).expect("symbol index") == release
        })
        .expect("real active-payload release relocation");
    let mut attack = bytes.to_vec();
    let info = u64::from_le_bytes(attack[record + 8..record + 16].try_into().expect("ELF64 info"));
    let replacement = (u64::try_from(clone).expect("small index") << 32) | (info & 0xffff_ffff);
    attack[record + 8..record + 16].copy_from_slice(&replacement.to_le_bytes());
    assert_eq!(
        zryna_backend_native::generic_owned_v2::validate_object_inventory(&attack, mir)
            .expect_err("release cannot be silently rebound to another admitted runtime operation")
            .code(),
        "ZRYNA-N7103"
    );
    zryna_backend_native::generic_owned_v2::validate_object_inventory(bytes, mir)
        .expect("pristine exact release inventory after hostile mutation");
}
