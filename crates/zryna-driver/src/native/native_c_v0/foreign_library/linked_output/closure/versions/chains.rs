//! Checked GNU chain traversal with separate definition and need record validation.
use super::{
    Diagnostic, Dynamic, MAX_STRINGS, Version, Versions, rejected, string, string_bytes, symbols,
    u16_word, u32_word, word,
};

pub(super) fn read(
    bytes: &[u8],
    count: usize,
    definition: bool,
    d: &Dynamic<'_>,
    out: &mut Versions,
    name_bytes: &mut usize,
) -> Result<(), Diagnostic> {
    let mut offset = 0_usize;
    let mut used = vec![false; bytes.len()];
    for index in 0..count {
        let header = if definition { 20 } else { 16 };
        claim(&mut used, offset, header)?;
        if word(bytes, offset, 2)? != 1 {
            return Err(rejected());
        }
        let (aux_count, aux_offset, next) = if definition {
            (
                word(bytes, offset + 6, 2)?,
                word(bytes, offset + 12, 4)?,
                word(bytes, offset + 16, 4)?,
            )
        } else {
            (word(bytes, offset + 2, 2)?, word(bytes, offset + 8, 4)?, word(bytes, offset + 12, 4)?)
        };
        if aux_count == 0 || aux_count > 128 || aux_offset < header as u64 {
            return Err(rejected());
        }
        let mut aux = offset
            .checked_add(usize::try_from(aux_offset).map_err(|_| rejected())?)
            .ok_or_else(rejected)?;
        let provider =
            if definition { None } else { Some(string(d.strings, word(bytes, offset + 4, 4)?)?) };
        if provider.as_ref().is_some_and(|p| !d.needed.contains(p)) {
            return Err(rejected());
        }
        let mut names = Vec::new();
        for position in 0..aux_count {
            let width = if definition { 8 } else { 16 };
            claim(&mut used, aux, width)?;
            let raw_name =
                string_bytes(d.strings, word(bytes, aux + if definition { 0 } else { 8 }, 4)?)?;
            let retained_size = raw_name
                .len()
                .checked_add(provider.as_ref().map_or(0, String::len))
                .ok_or_else(rejected)?;
            *name_bytes = name_bytes.checked_add(retained_size).ok_or_else(rejected)?;
            if *name_bytes > MAX_STRINGS {
                return Err(rejected());
            }
            let name = String::from_utf8(raw_name.to_vec()).map_err(|_| rejected())?;
            let step = word(bytes, aux + if definition { 4 } else { 12 }, 4)?;
            if definition {
                names.push(name);
            } else {
                need_record(bytes, aux, name, provider.as_deref(), out)?;
            }
            if position + 1 == aux_count {
                if step != 0 {
                    return Err(rejected());
                }
            } else {
                if step < width as u64 {
                    return Err(rejected());
                }
                aux = aux
                    .checked_add(usize::try_from(step).map_err(|_| rejected())?)
                    .ok_or_else(rejected)?;
            }
        }
        if definition {
            definition_record(bytes, offset, &names, out)?;
        }
        if index + 1 == count {
            if next != 0 {
                return Err(rejected());
            }
        } else {
            if next < header as u64 {
                return Err(rejected());
            }
            offset = offset
                .checked_add(usize::try_from(next).map_err(|_| rejected())?)
                .ok_or_else(rejected)?;
        }
    }
    // No hidden unparsed records can follow the supported chains (zero alignment padding allowed).
    if bytes.iter().zip(&used).any(|(b, marked)| !marked && *b != 0) {
        return Err(rejected());
    }
    for version in out.definitions.values() {
        if version.parents.iter().any(|parent| !out.definitions.values().any(|v| &v.name == parent))
        {
            return Err(rejected());
        }
    }
    Ok(())
}
fn claim(used: &mut [bool], offset: usize, size: usize) -> Result<(), Diagnostic> {
    let tail = used
        .get_mut(offset..offset.checked_add(size).ok_or_else(rejected)?)
        .ok_or_else(rejected)?;
    if tail.iter().any(|b| *b) {
        return Err(rejected());
    }
    tail.fill(true);
    Ok(())
}

fn need_record(
    bytes: &[u8],
    aux: usize,
    name: String,
    provider: Option<&str>,
    out: &mut Versions,
) -> Result<(), Diagnostic> {
    let id = u16_word(bytes, aux + 6, 2)?;
    let flags = u16_word(bytes, aux + 4, 2)?;
    if u32_word(bytes, aux, 4)? != symbols::sysv_hash(name.as_bytes())
        || id < 2
        || id & 0x8000 != 0
        || flags & !2 != 0
        || out.definitions.contains_key(&id)
        || out
            .needs
            .insert(
                id,
                Version {
                    name,
                    provider: provider.map(str::to_owned),
                    flags,
                    hash: u32_word(bytes, aux, 4)?,
                    parents: Vec::new(),
                },
            )
            .is_some()
    {
        return Err(rejected());
    }
    Ok(())
}
fn definition_record(
    bytes: &[u8],
    offset: usize,
    names: &[String],
    out: &mut Versions,
) -> Result<(), Diagnostic> {
    let id = u16_word(bytes, offset + 4, 2)?;
    let flags = u16_word(bytes, offset + 2, 2)?;
    if u32_word(bytes, offset + 8, 4)? != symbols::sysv_hash(names[0].as_bytes())
        || out.definitions.values().any(|v| v.name == names[0])
        || id == 0
        || id & 0x8000 != 0
        || flags & !3 != 0
        || out.needs.contains_key(&id)
        || out
            .definitions
            .insert(
                id,
                Version {
                    name: names[0].clone(),
                    provider: None,
                    flags,
                    hash: u32_word(bytes, offset + 8, 4)?,
                    parents: names[1..].to_vec(),
                },
            )
            .is_some()
    {
        return Err(rejected());
    }
    Ok(())
}
