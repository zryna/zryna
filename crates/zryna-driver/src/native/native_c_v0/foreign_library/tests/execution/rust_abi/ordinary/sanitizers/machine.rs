//! Bounded actual ELF/E8/PLT decoding for the private fixture, no machine-code execution.
use super::*;
use object::{Object as _, ObjectSection as _, ObjectSymbol as _, ObjectSymbolTable as _};
pub(super) fn parse(bytes: &[u8]) -> Result<object::File<'_>, Diagnostic> {
    if bytes.len() > native::MAX_NATIVE_EXECUTABLE_BYTES {
        return Err(rejected());
    }
    let file = object::File::parse(bytes).map_err(|_| rejected())?;
    if file.format() != object::BinaryFormat::Elf
        || !file.is_64()
        || !file.is_little_endian()
        || file.architecture() != object::Architecture::X86_64
        || file.kind() != object::ObjectKind::Executable
    {
        return Err(rejected());
    }
    Ok(file)
}
pub(super) fn text<'a>(file: &object::File<'a>, name: &str) -> Result<(u64, &'a [u8]), Diagnostic> {
    let mut symbols = file.symbols().filter(|s| s.name().ok() == Some(name));
    let symbol = symbols.next().ok_or_else(rejected)?;
    if symbols.next().is_some()
        || symbol.is_undefined()
        || symbol.kind() != object::SymbolKind::Text
        || symbol.size() == 0
    {
        return Err(rejected());
    }
    let section = file
        .section_by_index(symbol.section_index().ok_or_else(rejected)?)
        .map_err(|_| rejected())?;
    let data = section
        .data_range(symbol.address(), symbol.size())
        .map_err(|_| rejected())?
        .ok_or_else(rejected)?;
    Ok((symbol.address(), data))
}
fn location<'a>(
    file: &object::File<'a>,
    address: u64,
    size: u64,
) -> Result<(usize, &'a [u8]), Diagnostic> {
    let mut found = Vec::new();
    for section in file.sections() {
        if section.kind() != object::SectionKind::Text {
            continue;
        }
        let Some(delta) = address.checked_sub(section.address()) else { continue };
        if delta >= section.size() || size > section.size() - delta {
            continue;
        }
        let data =
            section.data_range(address, size).map_err(|_| rejected())?.ok_or_else(rejected)?;
        let (offset, _) = section.file_range().ok_or_else(rejected)?;
        found.push((
            usize::try_from(offset.checked_add(delta).ok_or_else(rejected)?)
                .map_err(|_| rejected())?,
            data,
        ));
    }
    if found.len() != 1 {
        return Err(rejected());
    }
    found.pop().ok_or_else(rejected)
}
fn relative(address: u64, length: u64, data: &[u8]) -> Result<u64, Diagnostic> {
    let displacement = i32::from_le_bytes(data.try_into().map_err(|_| rejected())?);
    address
        .checked_add(length)
        .and_then(|n| n.checked_add_signed(i64::from(displacement)))
        .ok_or_else(rejected)
}
pub(super) fn call(file: &object::File<'_>, address: u64) -> Result<u64, Diagnostic> {
    let (_, instruction) = location(file, address, 5)?;
    if instruction.first() != Some(&0xe8) {
        return Err(rejected());
    }
    relative(address, 5, &instruction[1..])
}
pub(super) fn hook(file: &object::File<'_>, address: u64, name: &str) -> Result<(), Diagnostic> {
    if let Some(import) = name.strip_suffix("@plt") {
        let (_, stub) = location(file, address, 6)?;
        if stub.get(..2) != Some(&[0xff, 0x25]) {
            return Err(rejected());
        }
        let slot = relative(address, 6, &stub[2..])?;
        let mut entries =
            file.dynamic_relocations().ok_or_else(rejected)?.filter(|(at, _)| *at == slot);
        let (_, relocation) = entries.next().ok_or_else(rejected)?;
        if entries.next().is_some()
            || relocation.flags() != (object::RelocationFlags::Elf { r_type: 7 })
            || relocation.addend() != 0
        {
            return Err(rejected());
        }
        let object::RelocationTarget::Symbol(index) = relocation.target() else {
            return Err(rejected());
        };
        let symbol = file
            .dynamic_symbol_table()
            .ok_or_else(rejected)?
            .symbol_by_index(index)
            .map_err(|_| rejected())?;
        if !symbol.is_undefined() || symbol.name().map_err(|_| rejected())? != import {
            return Err(rejected());
        }
    } else if text(file, name)?.0 != address {
        return Err(rejected());
    }
    Ok(())
}
pub(super) fn redirect(bytes: &[u8], callsite: u64) -> Result<Vec<u8>, Diagnostic> {
    let file = parse(bytes)?;
    let real = text(&file, "sum_bytes")?.0;
    let replacement = text(&file, "add")?.0;
    let (start, _) = location(&file, callsite, 5)?;
    if real == replacement || call(&file, callsite)? != real {
        return Err(rejected());
    }
    let delta = i128::from(replacement) - i128::from(callsite.checked_add(5).ok_or_else(rejected)?);
    let delta = i32::try_from(delta).map_err(|_| rejected())?;
    let mut changed = bytes.to_vec();
    changed
        .get_mut(
            start.checked_add(1).ok_or_else(rejected)?
                ..start.checked_add(5).ok_or_else(rejected)?,
        )
        .ok_or_else(rejected)?
        .copy_from_slice(&delta.to_le_bytes());
    // Both Rust functions must be untouched; this is solely a detached caller mutation.
    super::super::targets::compare_functions(bytes, &changed)?;
    Ok(changed)
}
