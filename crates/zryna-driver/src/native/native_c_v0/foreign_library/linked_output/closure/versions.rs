//! Bound GNU need/definition chains to their active dynamic entries and symbol table.
use super::{
    Diagnostic, Serialize,
    dynamic::Dynamic,
    image::{Image, MAX_STRINGS, string, string_bytes, u16_word, u32_word, word},
    rejected, symbols,
};
use std::collections::BTreeMap;
#[path = "versions/chains.rs"]
mod chains;

#[derive(Debug, Serialize)]
pub(crate) struct Version {
    pub(crate) name: String,
    pub(crate) provider: Option<String>,
    pub(crate) flags: u16,
    pub(crate) hash: u32,
    pub(crate) parents: Vec<String>,
}
#[derive(Default, Debug, Serialize)]
pub(crate) struct Versions {
    pub(crate) definitions: BTreeMap<u16, Version>,
    pub(crate) needs: BTreeMap<u16, Version>,
    pub(crate) symbols: Vec<u16>,
}
impl Versions {
    pub(super) fn name_bytes(&self) -> usize {
        self.definitions
            .values()
            .chain(self.needs.values())
            .map(|v| {
                v.name.len()
                    + v.provider.as_ref().map_or(0, String::len)
                    + v.parents.iter().map(String::len).sum::<usize>()
            })
            .sum()
    }
}

pub(super) fn read(elf: &Image<'_>, d: &Dynamic<'_>) -> Result<Versions, Diagnostic> {
    let mut result = Versions::default();
    let mut name_bytes = 0_usize;
    for (kind, address_tag, count_tag) in
        [(0x6fff_fffd, 0x6fff_fffc, 0x6fff_fffd), (0x6fff_fffe, 0x6fff_fffe, 0x6fff_ffff)]
    {
        let section = elf.unique(kind)?;
        match (section, d.tags.get(&address_tag), d.tags.get(&count_tag)) {
            (None, None, None) => {}
            (Some((_, s)), Some(&a), Some(&count))
                if a == s.address && count == s.info as u64 && count > 0 && count <= 256 =>
            {
                if s.link != elf.sections[d.symbol_section].link || s.size > 65_536 {
                    return Err(rejected());
                }
                chains::read(
                    elf.section(s)?,
                    usize::try_from(count).map_err(|_| rejected())?,
                    kind == 0x6fff_fffd,
                    d,
                    &mut result,
                    &mut name_bytes,
                )?;
            }
            _ => return Err(rejected()),
        }
    }
    match (elf.unique(0x6fff_ffff)?, d.tags.get(&0x6fff_fff0)) {
        (None, None) if result.definitions.is_empty() && result.needs.is_empty() => {}
        (Some((_, s)), Some(&a))
            if a == s.address
                && s.link == d.symbol_section
                && s.size == elf.sections[d.symbol_section].size / 12
                && s.entry_size == 2 =>
        {
            for row in elf.section(s)?.chunks_exact(2) {
                result.symbols.push(u16_word(row, 0, 2)?);
            }
        }
        _ => return Err(rejected()),
    }
    Ok(result)
}
