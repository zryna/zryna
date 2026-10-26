//! Closed dynamic-tag interpretation; preserve entries without claiming relocation safety.
use super::{
    Diagnostic,
    image::{Image, MAX_ROWS, MAX_STRINGS, MAX_SYMBOLS, string, word},
    rejected, symbols,
};
use std::collections::BTreeMap;

pub(super) struct Dynamic<'a> {
    pub entries: Vec<(u64, u64)>,
    pub tags: BTreeMap<u64, u64>,
    pub strings: &'a [u8],
    pub needed: Vec<String>,
    pub soname: Option<String>,
    pub symbol_section: usize,
}
pub(super) fn read<'a>(elf: &Image<'a>) -> Result<Dynamic<'a>, Diagnostic> {
    let mut entries = Vec::new();
    let mut tags = BTreeMap::new();
    let mut needed_offsets = Vec::new();
    let mut ended = false;
    for row in elf.dynamic.chunks_exact(16) {
        let tag = word(row, 0, 8)?;
        let value = word(row, 8, 8)?;
        if ended && (tag != 0 || value != 0) {
            return Err(rejected());
        }
        // RPATH/RUNPATH, FILTER/AUXILIARY/AUDIT, symbolic and textrel effects are unsupported.
        if !matches!(tag, 0..=14 | 17..=21 | 23..=28 | 30 | 32..=33 | 35..=37 |
            0x6fff_fef5 | 0x6fff_fff0 | 0x6fff_fff9..=0x6fff_ffff | 0x7000_0000 | 0x7000_0001 | 0x7000_0003)
        {
            return Err(rejected());
        }
        if tag == 0 {
            if value != 0 {
                return Err(rejected());
            }
            ended = true;
        } else if tag == 1 {
            needed_offsets.push(value);
        } else if tags.insert(tag, value).is_some() {
            return Err(rejected());
        }
        entries.push((tag, value));
    }
    if !ended || needed_offsets.len() > 32 {
        return Err(rejected());
    }
    let address = tags.get(&5).copied().ok_or_else(rejected)?;
    let size = tags.get(&10).copied().ok_or_else(rejected)?;
    if size == 0 || size > MAX_STRINGS as u64 {
        return Err(rejected());
    }
    let strings = elf.mapped(address, size)?;
    if strings.first() != Some(&0) || strings.last() != Some(&0) {
        return Err(rejected());
    }
    let (symbol_section, sym) = elf.unique(11)?.ok_or_else(rejected)?;
    let strings_section = elf.sections.get(sym.link).ok_or_else(rejected)?;
    if strings_section.kind != 3
        || strings_section.address != address
        || strings_section.size != size
        || elf.section(strings_section)? != strings
        || tags.get(&6) != Some(&sym.address)
        || tags.get(&11) != Some(&24)
        || sym.entry_size != 24
        || sym.size % 24 != 0
        || sym.size / 24 > MAX_SYMBOLS as u64
        || sym.size == 0
    {
        return Err(rejected());
    }
    elf.section(sym)?;
    let needed =
        needed_offsets.iter().map(|off| string(strings, *off)).collect::<Result<Vec<_>, _>>()?;
    let mut unique = std::collections::BTreeSet::new();
    if needed.iter().any(|s| !logical(s) || !unique.insert(s)) {
        return Err(rejected());
    }
    let soname = tags.get(&14).map(|off| string(strings, *off)).transpose()?;
    if soname.as_ref().is_some_and(|s| !logical(s)) {
        return Err(rejected());
    }
    let result = Dynamic { entries, tags, strings, needed, soname, symbol_section };
    tables(elf, &result)?;
    symbols::hash_tables(elf, &result, usize::try_from(sym.size / 24).map_err(|_| rejected())?)?;
    Ok(result)
}
pub(super) fn logical(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        && name != "."
        && name != ".."
}
fn tables(elf: &Image<'_>, d: &Dynamic<'_>) -> Result<(), Diagnostic> {
    for (address, size, entry, width) in [(7, 8, 9, 24), (17, 18, 19, 16), (36, 35, 37, 8)] {
        match (d.tags.get(&address), d.tags.get(&size), d.tags.get(&entry)) {
            (None, None, None) => {}
            (Some(&a), Some(&s), Some(&e))
                if e == width && s % width == 0 && s / width <= MAX_SYMBOLS as u64 =>
            {
                elf.mapped(a, s)?;
            }
            _ => return Err(rejected()),
        }
    }
    for (count_tag, size_tag, width) in [(0x6fff_fff9, 8, 24), (0x6fff_fffa, 18, 16)] {
        if let Some(count) = d.tags.get(&count_tag)
            && d.tags.get(&size_tag).is_none_or(|size| *count > size / width)
        {
            return Err(rejected());
        }
    }
    for (address, size) in [(25, 27), (26, 28), (32, 33)] {
        match (d.tags.get(&address), d.tags.get(&size)) {
            (None, None) => {}
            (Some(&a), Some(&s)) if s % 8 == 0 && s / 8 <= MAX_ROWS as u64 => {
                elf.mapped(a, s)?;
            }
            _ => return Err(rejected()),
        }
    }
    match (d.tags.get(&23), d.tags.get(&2), d.tags.get(&20)) {
        (None, None, None) => {}
        (Some(&a), Some(&s), Some(&7)) if s % 24 == 0 && s / 24 <= MAX_SYMBOLS as u64 => {
            elf.mapped(a, s)?;
        }
        _ => return Err(rejected()),
    }
    if d.tags.get(&30).is_some_and(|v| v & !(8 | 16) != 0)
        || d.tags.get(&0x6fff_fffb).is_some_and(|v| v & !(1 | 0x0800_0000) != 0)
    {
        return Err(rejected());
    }
    for tag in [12, 13] {
        if let Some(&address) = d.tags.get(&tag) {
            elf.mapped(address, 1)?;
        }
    }
    match (d.tags.get(&0x7000_0000), d.tags.get(&0x7000_0001), d.tags.get(&0x7000_0003)) {
        (None, None, None) => {}
        (Some(&a), Some(&s), Some(&e)) if e > 0 && s % e == 0 && s <= MAX_STRINGS as u64 => {
            elf.mapped(a, s)?;
        }
        _ => return Err(rejected()),
    }
    Ok(())
}
