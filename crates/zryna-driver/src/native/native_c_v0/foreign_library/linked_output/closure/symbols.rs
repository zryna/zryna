//! Dynamic symbols and supported hash table structure; kinds remain explicit observations.
use super::{
    Diagnostic, ProviderExpectation, Serialize,
    dynamic::Dynamic,
    image::{Image, MAX_STRINGS, MAX_SYMBOLS, index_word, string, string_bytes, u32_word, word},
    rejected, versions,
};
#[path = "symbols/hash.rs"]
mod hash;
pub(super) fn hash_tables(
    elf: &Image<'_>,
    d: &Dynamic<'_>,
    count: usize,
) -> Result<(), Diagnostic> {
    hash::check(elf, d, count)
}

#[derive(Debug, Serialize)]
pub(crate) struct Symbol {
    pub(crate) name: String,
    pub(crate) binding: u8,
    pub(crate) kind: u8,
    pub(crate) visibility: u8,
    pub(crate) defined: bool,
    pub(crate) value: u64,
    pub(crate) size: u64,
    pub(crate) version: u16,
    pub(crate) hidden_version: bool,
    pub(crate) lookup_reachable: bool,
}
pub(super) fn read(
    elf: &Image<'_>,
    d: &Dynamic<'_>,
    versions: &versions::Versions,
    policy: &ProviderExpectation,
) -> Result<Vec<Symbol>, Diagnostic> {
    let data = elf.section(&elf.sections[d.symbol_section])?;
    let mut result = Vec::new();
    let mut name_bytes = versions.name_bytes();
    let lookup = if let Some((_, s)) = elf.unique(0x6fff_fff6)? {
        let hash = elf.section(s)?;
        let header = 16 + index_word(hash, 8, 4)? * 8 + index_word(hash, 0, 4)? * 4;
        if hash.len() == header { 0..0 } else { index_word(hash, 4, 4)?..data.len() / 24 }
    } else {
        1..data.len() / 24
    };
    for (index, row) in data.chunks_exact(24).enumerate() {
        let binding = row[4] >> 4;
        let kind = row[4] & 15;
        let visibility = row[5];
        let section = index_word(row, 6, 2)?;
        let version = versions.symbols.get(index).copied().unwrap_or(1);
        let id = version & 0x7fff;
        if !matches!(binding, 0..=2)
            || !matches!(kind, 0..=2 | 6 | 10)
            || visibility > 3
            || (section == 0 && visibility != 0)
            || (binding == 2 && !policy.allow_weak)
            || (kind == 10 && !policy.allow_ifunc)
            || (kind == 6 && !policy.allow_tls)
            || (section >= elf.sections.len() && section != 0xfff1)
            || (id > 1
                && !versions.needs.contains_key(&id)
                && !versions.definitions.contains_key(&id))
            || (section == 0 && id > 1 && !versions.needs.contains_key(&id))
            || (version & 0x8000 != 0 && id < 2)
        {
            return Err(rejected());
        }
        if index == 0 {
            if row.iter().any(|b| *b != 0) || (version != 0 && !versions.symbols.is_empty()) {
                return Err(rejected());
            }
        } else if binding == 0 {
            return Err(rejected());
        }
        let offset = word(row, 0, 4)?;
        let raw_name = if offset == 0 { &[][..] } else { string_bytes(d.strings, offset)? };
        if index != 0 && raw_name.is_empty() {
            return Err(rejected());
        }
        name_bytes = name_bytes.checked_add(raw_name.len()).ok_or_else(rejected)?;
        if name_bytes > MAX_STRINGS {
            return Err(rejected());
        }
        let name = String::from_utf8(raw_name.to_vec()).map_err(|_| rejected())?;
        result.push(Symbol {
            name,
            binding,
            kind,
            visibility,
            defined: section != 0,
            value: word(row, 8, 8)?,
            size: word(row, 16, 8)?,
            version: id,
            hidden_version: version & 0x8000 != 0,
            lookup_reachable: lookup.contains(&index),
        });
    }
    Ok(result)
}

pub(super) fn gnu_hash(name: &[u8]) -> u32 {
    name.iter().fold(5381_u32, |a, b| a.wrapping_mul(33).wrapping_add(u32::from(*b)))
}
pub(super) fn sysv_hash(name: &[u8]) -> u32 {
    name.iter().fold(0_u32, |mut hash, b| {
        hash = hash.wrapping_shl(4).wrapping_add(u32::from(*b));
        let high = hash & 0xf000_0000;
        if high != 0 {
            hash ^= high >> 24;
        }
        hash & !high
    })
}
