//! Independent encoded object for audit tests; never claimed as C compiler output.
use object::{
    Architecture, BinaryFormat, Endianness, SectionKind, SymbolFlags, SymbolKind, SymbolScope,
    write::{Object, Symbol, SymbolSection},
};
pub(super) fn encoded_with_dependencies(dependencies: &[&str]) -> Vec<u8> {
    let mut object = Object::new(BinaryFormat::Elf, Architecture::X86_64, Endianness::Little);
    let text = object.section_id(object::write::StandardSection::Text);
    object.append_section_data(text, &[0xc3, 0xc3], 1);
    object.add_section(Vec::new(), b".note.GNU-stack".to_vec(), SectionKind::Other);
    for (offset, name) in
        ["sqlite_fixture_copy_bytes", "sqlite_fixture_release_bytes"].into_iter().enumerate()
    {
        object.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: u64::try_from(offset).expect("bounded symbol"),
            size: 1,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
    }
    for name in dependencies {
        object.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: 0,
            size: 0,
            kind: SymbolKind::Unknown,
            scope: SymbolScope::Dynamic,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
    }
    object.write().expect("independent closed audit object")
}
