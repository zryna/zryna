//! Independently bind SysV/GNU lookup tables to the exact active dynamic symbol data.
use super::*;

pub(super) fn check(elf: &Image<'_>, d: &Dynamic<'_>, count: usize) -> Result<(), Diagnostic> {
    let mut present = false;
    if let Some(data) = table(elf, d, 4, 5)? {
        present = true;
        sysv(elf, d, count, data)?;
    }
    if let Some(data) = table(elf, d, 0x6fff_fef5, 0x6fff_fff6)? {
        present = true;
        gnu(elf, d, count, data)?;
    }
    if !present {
        return Err(rejected());
    }
    Ok(())
}
fn table<'a>(
    elf: &Image<'a>,
    d: &Dynamic<'_>,
    tag: u64,
    kind: u32,
) -> Result<Option<&'a [u8]>, Diagnostic> {
    match (d.tags.get(&tag), elf.unique(kind)?) {
        (None, None) => Ok(None),
        (Some(address), Some((_, section)))
            if section.address == *address && section.link == d.symbol_section =>
        {
            Ok(Some(elf.section(section)?))
        }
        _ => Err(rejected()),
    }
}
fn sysv(elf: &Image<'_>, d: &Dynamic<'_>, count: usize, data: &[u8]) -> Result<(), Diagnostic> {
    let buckets = index_word(data, 0, 4)?;
    let chains = index_word(data, 4, 4)?;
    if buckets == 0
        || buckets > MAX_SYMBOLS
        || chains != count
        || data.len() != 8 + (buckets + chains) * 4
    {
        return Err(rejected());
    }
    if word(data, 8 + buckets * 4, 4)? != 0 {
        return Err(rejected());
    }
    let mut seen = vec![false; count];
    for bucket in 0..buckets {
        let mut index = index_word(data, 8 + bucket * 4, 4)?;
        while index != 0 {
            if index >= count || seen[index] {
                return Err(rejected());
            }
            seen[index] = true;
            let name = symbol_name(elf, d, index)?;
            if sysv_hash(name.as_bytes()) as usize % buckets != bucket {
                return Err(rejected());
            }
            index = index_word(data, 8 + (buckets + index) * 4, 4)?;
        }
    }
    if seen[1..].iter().any(|s| !s) {
        return Err(rejected());
    }
    Ok(())
}
fn gnu(elf: &Image<'_>, d: &Dynamic<'_>, count: usize, data: &[u8]) -> Result<(), Diagnostic> {
    let buckets = index_word(data, 0, 4)?;
    let start = index_word(data, 4, 4)?;
    let bloom = index_word(data, 8, 4)?;
    let shift = u32_word(data, 12, 4)?;
    if buckets == 0
        || buckets > MAX_SYMBOLS
        || start > count
        || start == 0
        || bloom == 0
        || bloom > MAX_SYMBOLS
        || !bloom.is_power_of_two()
        || shift >= 32
    {
        return Err(rejected());
    }
    let bucket_base = 16 + bloom * 8;
    let chain_base = bucket_base + buckets * 4;
    let symbols = elf.section(&elf.sections[d.symbol_section])?;
    if symbols.chunks_exact(24).take(start).skip(1).any(lookup_definition) {
        return Err(rejected());
    }
    if data.len() == chain_base {
        // Empty lookup is valid for import-only executables, never omitted exports.
        if data[16..].iter().any(|byte| *byte != 0) {
            return Err(rejected());
        }
        if symbols.chunks_exact(24).skip(1).any(lookup_definition) {
            return Err(rejected());
        }
        return Ok(());
    }
    if data.len() != chain_base + (count - start) * 4 {
        return Err(rejected());
    }
    let mut seen = vec![false; count];
    for bucket in 0..buckets {
        let mut index = index_word(data, bucket_base + bucket * 4, 4)?;
        if index == 0 {
            continue;
        }
        loop {
            if index < start || index >= count || seen[index] {
                return Err(rejected());
            }
            seen[index] = true;
            let hash = gnu_hash(symbol_name(elf, d, index)?.as_bytes());
            let value = u32_word(data, chain_base + (index - start) * 4, 4)?;
            let filter = word(data, 16 + ((hash as usize / 64) & (bloom - 1)) * 8, 8)?;
            let bits = (1_u64 << (hash % 64)) | (1_u64 << ((hash >> shift) % 64));
            if value | 1 != hash | 1 || hash as usize % buckets != bucket || filter & bits != bits {
                return Err(rejected());
            }
            if value & 1 != 0 {
                break;
            }
            index += 1;
        }
    }
    if seen[start..].iter().any(|s| !s) {
        return Err(rejected());
    }
    Ok(())
}
fn lookup_definition(row: &[u8]) -> bool {
    (row[6] != 0 || row[7] != 0) && matches!(row[4] >> 4, 1 | 2) && matches!(row[5], 0 | 3)
}
fn symbol_name(elf: &Image<'_>, d: &Dynamic<'_>, index: usize) -> Result<String, Diagnostic> {
    let data = elf.section(&elf.sections[d.symbol_section])?;
    string(d.strings, word(data, index.checked_mul(24).ok_or_else(rejected)?, 4)?)
}
