//! Actual ordinary ELF mutations; raw claims cannot create a positive program owner.
use object::{Object, ObjectSection, ObjectSymbol};
use zryna_backend_native::generic_execution_v1::validate_object_inventory;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("ELF64 word"))
}
fn put(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn reject(bytes: &[u8], p: &VerifiedProgram<'_, '_>, name: &str, edit: impl FnOnce(&mut [u8])) {
    let mut changed = bytes.to_vec();
    edit(&mut changed);
    let failure = validate_object_inventory(&changed, p).expect_err(name);
    assert_eq!(failure.code, "ZRYNA-N7103", "{name}");
}
pub(super) fn check(bytes: &[u8], p: &VerifiedProgram<'_, '_>) {
    let file = object::File::parse(bytes).expect("pristine genuine object");
    let tls = file.symbols().find(|s| s.name().ok() == Some(super::TLS)).expect("TLS symbol");
    let stub = file.symbols().find(|s| s.name().ok() == Some(super::STUB)).expect("stub");
    let section = file.section_by_name(".tbss").expect("TLS section");
    let text = file.section_by_name(".text").expect("text");
    let symtab = file.section_by_name(".symtab").expect("symbol table");
    let rela = file.section_by_name(".rela.text").expect("actual relocations");
    let word = |n| usize::try_from(n).expect("bounded ELF offset");
    let symbol_at = word(symtab.file_range().expect("symbol bytes").0) + tls.index().0 * 24;
    let stub_at = word(symtab.file_range().expect("symbol bytes").0) + stub.index().0 * 24;
    let shoff = word(u64_at(bytes, 40));
    let shsize = usize::from(u16::from_le_bytes(bytes[58..60].try_into().expect("ELF64 header")));
    let section_at = shoff + section.index().0 * shsize;
    let code = word(text.file_range().expect("text bytes").0) + word(stub.address());
    let (rel_begin, rel_size) = rela.file_range().expect("RELA bytes");
    let relocation_at = (word(rel_begin)..word(rel_begin + rel_size))
        .step_by(24)
        .find(|offset| {
            (u64_at(bytes, offset + 8) & u64::from(u32::MAX))
                == u64::from(object::elf::R_X86_64_TPOFF32)
        })
        .expect("exact local-exec relocation");
    inventory(bytes, p, &file, symbol_at, section_at);
    geometry(bytes, p, symbol_at, section_at, stub_at, code);
    relocations(bytes, p, relocation_at, stub.address(), stub.index().0);
}
fn geometry(
    bytes: &[u8],
    p: &VerifiedProgram<'_, '_>,
    symbol_at: usize,
    section_at: usize,
    stub_at: usize,
    code: usize,
) {
    reject(bytes, p, "wrong TLS allocation size", |b| put(b, symbol_at + 16, 33_554_416));
    reject(bytes, p, "wrong TLS symbol type", |b| b[symbol_at + 4] = 1);
    reject(bytes, p, "public TLS symbol", |b| b[symbol_at + 4] = 0x16);
    reject(bytes, p, "missing TLS section flag", |b| put(b, section_at + 8, 3));
    reject(bytes, p, "wrong TLS section extent", |b| put(b, section_at + 32, 33_554_416));
    reject(bytes, p, "wrong TLS alignment", |b| put(b, section_at + 48, 8));
    reject(bytes, p, "wrong stub extent", |b| put(b, stub_at + 16, 16));
    reject(bytes, p, "wrong thread-pointer segment", |b| b[code] = 0x65);
    reject(bytes, p, "wrong TLS displacement byte", |b| b[code + 12] = 1);
}
fn relocations(
    bytes: &[u8],
    p: &VerifiedProgram<'_, '_>,
    relocation_at: usize,
    stub_address: u64,
    stub_index: usize,
) {
    let info = u64_at(bytes, relocation_at + 8);
    reject(bytes, p, "general-dynamic TLS forbidden", |b| {
        put(b, relocation_at + 8, (info & !0xffff_ffff) | u64::from(object::elf::R_X86_64_TLSGD));
    });
    reject(bytes, p, "wrong TLS relocation field", |b| {
        put(b, relocation_at, stub_address + 13);
    });
    reject(bytes, p, "wrong TLS relocation target", |b| {
        put(
            b,
            relocation_at + 8,
            (u64::try_from(stub_index).expect("symbol index") << 32)
                | u64::from(object::elf::R_X86_64_TPOFF32),
        );
    });
    reject(bytes, p, "wrong TLS addend", |b| put(b, relocation_at + 16, 1));
    reject(bytes, p, "missing TLS relocation", |b| {
        put(b, relocation_at + 8, info & !0xffff_ffff);
    });
}

fn inventory(
    bytes: &[u8],
    p: &VerifiedProgram<'_, '_>,
    file: &object::File<'_>,
    symbol_at: usize,
    section_at: usize,
) {
    let symtab = file.section_by_name(".symtab").expect("symbols");
    let extra = file
        .symbols()
        .find(|s| s.kind() == object::SymbolKind::Text && s.name().ok() != Some(super::STUB))
        .expect("real additional symbol");
    let extra_at = usize::try_from(symtab.file_range().expect("symbols").0)
        .expect("bounded symbol table")
        + extra.index().0 * 24;
    reject(bytes, p, "missing TLS symbol", |b| b[symbol_at..symbol_at + 24].fill(0));
    reject(bytes, p, "extra TLS symbol", |b| {
        let duplicate = b[symbol_at..symbol_at + 24].to_vec();
        b[extra_at..extra_at + 24].copy_from_slice(&duplicate);
    });
    reject(bytes, p, "extra TLS storage", |b| put(b, section_at + 32, 33_554_480));
}
