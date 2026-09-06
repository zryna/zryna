use std::{
    collections::BTreeSet, fs, io::Read as _, os::unix::fs::OpenOptionsExt as _, path::Path,
};

use object::{BinaryFormat, Endianness, Object, ObjectKind, ObjectSection, ObjectSymbol};
use zryna_diagnostics::Diagnostic;

use super::super::{native_error, regular_file_identity, tool_identity_from_metadata};

pub(super) fn read_stable_file(path: &Path, maximum: usize) -> Result<Box<[u8]>, Diagnostic> {
    let before = regular_file_identity(path).map_err(|()| ownership_object_audit_error())?;
    if before.length == 0 || usize::try_from(before.length).map_or(true, |size| size > maximum) {
        return Err(ownership_object_audit_error());
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ownership_object_audit_error())?;
    let opened =
        tool_identity_from_metadata(&file.metadata().map_err(|_| ownership_object_audit_error())?)
            .map_err(|()| ownership_object_audit_error())?;
    if opened != before {
        return Err(ownership_object_audit_error());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(before.length).unwrap_or(0));
    file.read_to_end(&mut bytes).map_err(|_| ownership_object_audit_error())?;
    if regular_file_identity(path).map_err(|()| ownership_object_audit_error())? != before
        || bytes.len() != usize::try_from(before.length).unwrap_or(usize::MAX)
    {
        return Err(ownership_object_audit_error());
    }
    Ok(bytes.into_boxed_slice())
}

pub(super) fn audit_runtime_object(
    bytes: &[u8],
    mir: &zryna_native_mir::data_ownership_v1::VerifiedMirModule,
) -> Result<(), Diagnostic> {
    let file = object::File::parse(bytes).map_err(|_| ownership_object_audit_error())?;
    if file.format() != BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != Endianness::Little
        || file.kind() != ObjectKind::Relocatable
        || !file.is_64()
    {
        return Err(ownership_object_audit_error());
    }
    let approved_sections = BTreeSet::from([
        ".text",
        ".rela.text",
        ".data",
        ".bss",
        ".note.GNU-stack",
        ".note.gnu.property",
        ".symtab",
        ".strtab",
        ".shstrtab",
    ]);
    if file
        .sections()
        .any(|section| section.name().map_or(true, |name| !approved_sections.contains(name)))
    {
        return Err(ownership_object_audit_error());
    }
    let expected = mir.runtime_symbols().collect::<BTreeSet<_>>();
    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for symbol in file.symbols() {
        if symbol.is_undefined() {
            let name = symbol.name().map_err(|_| ownership_object_audit_error())?;
            if !name.is_empty() {
                undefined.insert(name);
            }
        } else if symbol.is_global() {
            let name = symbol.name().map_err(|_| ownership_object_audit_error())?;
            if !name.is_empty() {
                defined.insert(name);
            }
        }
    }
    if defined != expected
        || !undefined.iter().all(|name| ["free", "malloc", "memcpy", "memset"].contains(name))
    {
        return Err(ownership_object_audit_error());
    }
    for section in file.sections() {
        for (_, relocation) in section.relocations() {
            let object::RelocationTarget::Symbol(index) = relocation.target() else {
                return Err(ownership_object_audit_error());
            };
            let target = file.symbol_by_index(index).map_err(|_| ownership_object_audit_error())?;
            let name = target.name().map_err(|_| ownership_object_audit_error())?;
            if target.kind() != object::SymbolKind::Section
                && !defined.contains(name)
                && !undefined.contains(name)
                && !name.is_empty()
            {
                return Err(ownership_object_audit_error());
            }
        }
    }
    Ok(())
}

pub(super) fn audit_completed_executable(
    bytes: &[u8],
    symbol: &str,
    mir: &zryna_native_mir::data_ownership_v1::VerifiedMirModule,
) -> Result<(), Diagnostic> {
    let file = object::File::parse(bytes).map_err(|_| ownership_executable_audit_error())?;
    let defined = file
        .symbols()
        .filter(|item| !item.is_undefined())
        .filter_map(|item| item.name().ok())
        .collect::<BTreeSet<_>>();
    if !defined.contains(symbol)
        || !defined.contains("main")
        || mir.runtime_symbols().any(|runtime| !defined.contains(runtime))
    {
        return Err(ownership_executable_audit_error());
    }
    let approved_imports = [
        "",
        "__gmon_start__",
        "__libc_start_main",
        "fflush",
        "free",
        "fwrite",
        "malloc",
        "memcpy",
        "memset",
        "stdout",
    ];
    if file.symbols().filter(ObjectSymbol::is_undefined).any(|item| {
        item.name().map_or(true, |name| {
            let base = name.split_once('@').map_or(name, |(base, _)| base);
            !approved_imports.contains(&base)
        })
    }) {
        return Err(ownership_executable_audit_error());
    }
    Ok(())
}

fn ownership_object_audit_error() -> Diagnostic {
    native_error(
        "ZRYNA-N3402",
        "ownership runtime object failed its closed Linux x86-64 audit",
        "use the documented GNU toolchain and report the smallest reproducible program",
    )
}

pub(super) fn ownership_link_error() -> Diagnostic {
    native_error(
        "ZRYNA-N3403",
        "the validated GNU toolchain rejected ownership executable linking",
        "use the documented GNU toolchain and report the smallest reproducible program",
    )
}

fn ownership_executable_audit_error() -> Diagnostic {
    native_error(
        "ZRYNA-N3404",
        "completed ownership executable failed its closed Linux x86-64 audit",
        "use the documented GNU toolchain and report the smallest reproducible program",
    )
}
