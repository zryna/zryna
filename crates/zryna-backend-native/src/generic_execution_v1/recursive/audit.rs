//! Independent exact local-exec storage/stub/relocation expectations, not emitter claims.
use super::{STUB_NAME, TLS_NAME};
use crate::generic_execution_v1::audit::error;
use object::{
    Object, ObjectSection, ObjectSymbol, RelocationFlags, RelocationTarget, SectionFlags,
    SectionKind, SymbolFlags, SymbolKind, SymbolScope, SymbolSection,
};
use zryna_diagnostics::Diagnostic;

pub(in crate::generic_execution_v1) fn section(
    section: &object::Section<'_, '_>,
    required: bool,
) -> Result<(), Diagnostic> {
    let bytes = zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS
        .checked_mul(32)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(error)?;
    if !required
        || section.kind() != SectionKind::UninitializedTls
        || section.flags() != (SectionFlags::Elf { sh_flags: 0x403 })
        || section.size() != bytes
        || section.align() != 16
        || section.file_range().is_some()
        || !section.data().map_err(|_| error())?.is_empty()
        || section.relocations().next().is_some()
    {
        return Err(error());
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn symbol(
    file: &object::File<'_>,
    symbol: &object::Symbol<'_, '_>,
    required: bool,
) -> Result<(), Diagnostic> {
    let section = file.section_by_name(".tbss").ok_or_else(error)?;
    let bytes = zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS
        .checked_mul(32)
        .and_then(|n| n.checked_add(16))
        .ok_or_else(error)?;
    if !required
        || symbol.name().map_err(|_| error())? != TLS_NAME
        || symbol.kind() != SymbolKind::Tls
        || symbol.scope() != SymbolScope::Compilation
        || symbol.flags() != (SymbolFlags::Elf { st_info: 6, st_other: 0 })
        || symbol.section() != SymbolSection::Section(section.index())
        || symbol.address() != 0
        || symbol.size() != bytes
        || symbol.is_weak()
    {
        return Err(error());
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn stub(file: &object::File<'_>) -> Result<u64, Diagnostic> {
    let symbol = file.symbols().find(|s| s.name().ok() == Some(STUB_NAME)).ok_or_else(error)?;
    let text = file.section_by_name(".text").ok_or_else(error)?;
    let begin = usize::try_from(symbol.address()).map_err(|_| error())?;
    let end = begin.checked_add(17).ok_or_else(error)?;
    // Linux static local-exec: fs:0 is the thread pointer; link-time TPOFF32
    // resolves one local TLS block. No resolver call or lazy allocation exists.
    let expected: [u8; 17] =
        [0x64, 0x48, 0x8b, 0x04, 0x25, 0, 0, 0, 0, 0x48, 0x8d, 0x80, 0, 0, 0, 0, 0xc3];
    if symbol.size() != 17
        || text.data().map_err(|_| error())?.get(begin..end) != Some(expected.as_slice())
    {
        return Err(error());
    }
    Ok(symbol.address())
}
pub(in crate::generic_execution_v1) fn relocation(
    file: &object::File<'_>,
    offset: u64,
    relocation: &object::Relocation,
    required: bool,
) -> Result<(), Diagnostic> {
    if !required
        || relocation.flags() != (RelocationFlags::Elf { r_type: object::elf::R_X86_64_TPOFF32 })
        || relocation.addend() != 0
        || relocation.has_implicit_addend()
        || relocation.subtractor().is_some()
        || offset != stub(file)?.checked_add(12).ok_or_else(error)?
    {
        return Err(error());
    }
    let RelocationTarget::Symbol(target) = relocation.target() else {
        return Err(error());
    };
    let target = file.symbol_by_index(target).map_err(|_| error())?;
    symbol(file, &target, true)
}
