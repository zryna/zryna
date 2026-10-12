//! Raw bounded ELF tables before semantic Rust-profile symbol interpretation.

use super::super::audit::{MAX_SECTIONS, MAX_SYMBOLS, audit_error, limit_error};
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;

mod symbols;

pub(super) fn number(bytes: &[u8], offset: usize, width: usize) -> Result<u64, Diagnostic> {
    let end = offset.checked_add(width).ok_or_else(audit_error)?;
    let input = bytes.get(offset..end).ok_or_else(audit_error)?;
    let mut value = [0; 8];
    value.get_mut(..width).ok_or_else(audit_error)?.copy_from_slice(input);
    Ok(u64::from_le_bytes(value))
}
fn index(value: u64) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| audit_error())
}
fn data(bytes: &[u8], offset: u64, length: u64) -> Result<&[u8], Diagnostic> {
    let end = offset.checked_add(length).ok_or_else(audit_error)?;
    bytes.get(index(offset)?..index(end)?).ok_or_else(audit_error)
}
pub(super) fn name(strings: &[u8], offset: u64) -> Result<&str, Diagnostic> {
    let remaining = strings.get(index(offset)?..).ok_or_else(audit_error)?;
    let end = remaining.iter().take(129).position(|byte| *byte == 0).ok_or_else(audit_error)?;
    if end == 0 || end > 128 || !remaining[..end].iter().all(u8::is_ascii_graphic) {
        return Err(audit_error());
    }
    std::str::from_utf8(&remaining[..end]).map_err(|_| audit_error())
}

pub(super) struct Section<'a> {
    pub(super) ordinal: usize,
    pub(super) name: &'a str,
    pub(super) size: u64,
    pub(super) link: usize,
    pub(super) info: usize,
    pub(super) bytes: &'a [u8],
}

pub(super) fn check(bytes: &[u8], definitions: &BTreeSet<String>) -> Result<(), Diagnostic> {
    if !definitions.iter().map(String::as_str).eq(["add", "sum_bytes"])
        || bytes.get(..16) != Some(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        || number(bytes, 16, 2)? != 1
        || number(bytes, 18, 2)? != 62
        || number(bytes, 20, 4)? != 1
        || number(bytes, 24, 8)? != 0
        || number(bytes, 32, 8)? != 0
        || number(bytes, 48, 4)? != 0
        || number(bytes, 52, 2)? != 64
        || number(bytes, 54, 2)? != 0
        || number(bytes, 56, 2)? != 0
        || number(bytes, 58, 2)? != 64
    {
        return Err(audit_error());
    }
    let count = index(number(bytes, 60, 2)?)?;
    if count > MAX_SECTIONS + 1 {
        return Err(limit_error());
    }
    let table_offset = number(bytes, 40, 8)?;
    let table_bytes =
        u64::try_from(count.checked_mul(64).ok_or_else(audit_error)?).map_err(|_| audit_error())?;
    if count != 8 || table_offset < 64 || table_offset % 8 != 0 {
        return Err(audit_error());
    }
    let table = data(bytes, table_offset, table_bytes)?;
    if index(table_offset.checked_add(table_bytes).ok_or_else(audit_error)?)? != bytes.len()
        || table.get(..64).is_none_or(|null| null.iter().any(|byte| *byte != 0))
    {
        return Err(audit_error());
    }
    let strings_index = index(number(bytes, 62, 2)?)?;
    if strings_index == 0 || strings_index >= count {
        return Err(audit_error());
    }
    let string_header = &table[strings_index * 64..(strings_index + 1) * 64];
    let strings = data(bytes, number(string_header, 24, 8)?, number(string_header, 32, 8)?)?;
    if strings.first() != Some(&0) || strings.last() != Some(&0) {
        return Err(audit_error());
    }
    let mut sections = BTreeMap::new();
    let mut ranges = vec![(0, 64), (table_offset, table_offset + table_bytes)];
    for (ordinal, header) in table.chunks_exact(64).enumerate().skip(1) {
        let section_name = name(strings, number(header, 0, 4)?)?;
        let section = section(bytes, header, ordinal, section_name)?;
        let offset = number(header, 24, 8)?;
        if section.size != 0 {
            ranges.push((offset, offset.checked_add(section.size).ok_or_else(audit_error)?));
        }
        if sections.insert(section_name, section).is_some() {
            return Err(audit_error());
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0)
        || sections.get(".strtab").is_none_or(|section| section.ordinal != strings_index)
        || [".text", ".comment", ".note.GNU-stack", ".symtab"]
            .iter()
            .any(|key| !sections.contains_key(key))
    {
        return Err(audit_error());
    }
    let symbol_table = sections.get(".symtab").ok_or_else(audit_error)?;
    if symbol_table.link != strings_index
        || symbol_table.info != 2
        || symbol_table.bytes.len() / 24 > MAX_SYMBOLS + 1
    {
        return Err(audit_error());
    }
    symbols::check(symbol_table.bytes, strings, &sections, definitions)
}

fn section<'a>(
    bytes: &'a [u8],
    header: &[u8],
    ordinal: usize,
    name: &'a str,
) -> Result<Section<'a>, Diagnostic> {
    let kind = number(header, 4, 4)?;
    let flags = number(header, 8, 8)?;
    let size = number(header, 32, 8)?;
    let alignment = number(header, 48, 8)?;
    let entry = number(header, 56, 8)?;
    let (expected_kind, expected_flags, expected_alignment, expected_entry) = match name {
        ".strtab" => (3, 0, 1, 0),
        ".symtab" => (2, 0, 8, 24),
        ".comment" => (1, 48, 1, 1),
        ".note.GNU-stack" => (1, 0, 1, 0),
        ".text" => (1, 6, 4, 0),
        ".text.add" | ".text.sum_bytes" => (1, 6, 16, 0),
        _ => return Err(audit_error()),
    };
    if kind != expected_kind
        || flags != expected_flags
        || alignment != expected_alignment
        || entry != expected_entry
        || number(header, 16, 8)? != 0
        || number(header, 24, 8)? % alignment != 0
        || (matches!(name, ".text" | ".note.GNU-stack") && size != 0)
        || (matches!(name, ".text.add" | ".text.sum_bytes") && size == 0)
    {
        return Err(audit_error());
    }
    let link = index(number(header, 40, 4)?)?;
    let info = index(number(header, 44, 4)?)?;
    if name != ".symtab" && (link != 0 || info != 0) {
        return Err(audit_error());
    }
    let contents = data(bytes, number(header, 24, 8)?, size)?;
    if name == ".comment"
        && (!contents.starts_with(b"\0rustc version ")
            || contents.last() != Some(&0)
            || contents.len() > 128
            || !contents[1..contents.len() - 1]
                .iter()
                .all(|byte| byte.is_ascii_graphic() || *byte == b' '))
    {
        return Err(audit_error());
    }
    Ok(Section { ordinal, name, size, link, info, bytes: contents })
}
