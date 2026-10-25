//! ELF64 little-endian section/load mapping; unsupported stripped/extended forms reject.
use super::{Diagnostic, rejected};

pub(super) const MAX_ROWS: usize = 8192;
pub(super) const MAX_SYMBOLS: usize = 16_384;
pub(super) const MAX_STRINGS: usize = 1_048_576;

#[derive(Clone, Debug)]
pub(super) struct Section {
    pub kind: u32,
    pub address: u64,
    pub offset: u64,
    pub size: u64,
    pub link: usize,
    pub info: usize,
    pub entry_size: u64,
}
pub(super) struct Image<'a> {
    pub bytes: &'a [u8],
    pub sections: Vec<Section>,
    pub loads: Vec<(u64, u64, u64, u64)>,
    pub dynamic: &'a [u8],
    pub interpreter: Option<String>,
}

pub(super) fn range(bytes: &[u8], offset: u64, size: u64) -> Result<&[u8], Diagnostic> {
    let start = usize::try_from(offset).map_err(|_| rejected())?;
    let size = usize::try_from(size).map_err(|_| rejected())?;
    bytes.get(start..start.checked_add(size).ok_or_else(rejected)?).ok_or_else(rejected)
}
pub(super) fn word(bytes: &[u8], offset: usize, size: usize) -> Result<u64, Diagnostic> {
    let bytes = range(bytes, offset as u64, size as u64)?;
    Ok(bytes.iter().enumerate().fold(0, |a, (i, b)| a | (u64::from(*b) << (i * 8))))
}
pub(super) fn index_word(bytes: &[u8], offset: usize, size: usize) -> Result<usize, Diagnostic> {
    usize::try_from(word(bytes, offset, size)?).map_err(|_| rejected())
}
pub(super) fn u16_word(bytes: &[u8], offset: usize, size: usize) -> Result<u16, Diagnostic> {
    u16::try_from(word(bytes, offset, size)?).map_err(|_| rejected())
}
pub(super) fn u32_word(bytes: &[u8], offset: usize, size: usize) -> Result<u32, Diagnostic> {
    u32::try_from(word(bytes, offset, size)?).map_err(|_| rejected())
}
pub(super) fn string(bytes: &[u8], offset: u64) -> Result<String, Diagnostic> {
    String::from_utf8(string_bytes(bytes, offset)?.to_vec()).map_err(|_| rejected())
}
pub(super) fn string_bytes(bytes: &[u8], offset: u64) -> Result<&[u8], Diagnostic> {
    let start = usize::try_from(offset).map_err(|_| rejected())?;
    let tail = bytes.get(start..).ok_or_else(rejected)?;
    let end = tail.iter().position(|b| *b == 0).ok_or_else(rejected)?;
    if end == 0 || end > 4096 || tail[..end].iter().any(|b| *b < 32 || *b > 126) {
        return Err(rejected());
    }
    Ok(&tail[..end])
}
impl<'a> Image<'a> {
    pub(super) fn read(bytes: &'a [u8], kind: u16) -> Result<Self, Diagnostic> {
        if bytes.len() > 33_554_432
            || bytes.get(..7) != Some(b"\x7fELF\x02\x01\x01")
            || !matches!(bytes.get(7), Some(0 | 3))
            || bytes.get(8..16) != Some(&[0; 8])
            || word(bytes, 16, 2)? != u64::from(kind)
            || word(bytes, 18, 2)? != 62
            || word(bytes, 20, 4)? != 1
            || word(bytes, 52, 2)? != 64
            || word(bytes, 54, 2)? != 56
            || word(bytes, 58, 2)? != 64
        {
            return Err(rejected());
        }
        let phnum = usize::try_from(word(bytes, 56, 2)?).map_err(|_| rejected())?;
        let shnum = usize::try_from(word(bytes, 60, 2)?).map_err(|_| rejected())?;
        if phnum == 0 || phnum > 128 || shnum == 0 || shnum > MAX_ROWS {
            return Err(rejected());
        }
        let ph = range(bytes, word(bytes, 32, 8)?, (phnum * 56) as u64)?;
        let mut loads = Vec::new();
        let mut dynamic = None;
        let mut dynamic_mapping = None;
        let mut interpreter = None;
        for row in ph.chunks_exact(56) {
            let tag = word(row, 0, 4)?;
            let offset = word(row, 8, 8)?;
            let address = word(row, 16, 8)?;
            let size = word(row, 32, 8)?;
            let memory = word(row, 40, 8)?;
            if size > memory || address.checked_add(memory).is_none() {
                return Err(rejected());
            }
            let data = range(bytes, offset, size)?;
            if tag == 1 {
                if loads
                    .iter()
                    .any(|&(base, _, _, length)| address < base + length && base < address + memory)
                {
                    return Err(rejected());
                }
                loads.push((address, offset, size, memory));
            } else if tag == 2 {
                if size == 0 || size > 65_536 || size % 16 != 0 || dynamic.replace(data).is_some() {
                    return Err(rejected());
                }
                dynamic_mapping = Some((address, offset, size));
            } else if tag == 3 {
                let name = string(data, 0)?;
                if name.len() + 1 != data.len()
                    || !name.starts_with('/')
                    || interpreter.replace(name).is_some()
                {
                    return Err(rejected());
                }
            }
        }
        let sh = range(bytes, word(bytes, 40, 8)?, (shnum * 64) as u64)?;
        let mut sections = Vec::with_capacity(shnum);
        for row in sh.chunks_exact(64) {
            let section = Section {
                kind: u32::try_from(word(row, 4, 4)?).map_err(|_| rejected())?,
                address: word(row, 16, 8)?,
                offset: word(row, 24, 8)?,
                size: word(row, 32, 8)?,
                link: usize::try_from(word(row, 40, 4)?).map_err(|_| rejected())?,
                info: usize::try_from(word(row, 44, 4)?).map_err(|_| rejected())?,
                entry_size: word(row, 56, 8)?,
            };
            if section.link >= shnum
                || (section.kind != 8 && range(bytes, section.offset, section.size).is_err())
            {
                return Err(rejected());
            }
            sections.push(section);
        }
        let image =
            Self { bytes, sections, loads, dynamic: dynamic.ok_or_else(rejected)?, interpreter };
        // The active dynamic table must be exactly backed by one load, not an unrelated section.
        let (address, offset, size) = dynamic_mapping.ok_or_else(rejected)?;
        if image.file_offset(address, size)? != offset {
            return Err(rejected());
        }
        Ok(image)
    }
    pub(super) fn mapped(&self, address: u64, size: u64) -> Result<&'a [u8], Diagnostic> {
        range(self.bytes, self.file_offset(address, size)?, size)
    }
    fn file_offset(&self, address: u64, size: u64) -> Result<u64, Diagnostic> {
        let matching = self
            .loads
            .iter()
            .filter_map(|&(base, off, len, _)| {
                let delta = address.checked_sub(base)?;
                (delta <= len && size <= len - delta).then_some(off.checked_add(delta)?)
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(rejected());
        }
        Ok(matching[0])
    }
    pub(super) fn unique(&self, kind: u32) -> Result<Option<(usize, &Section)>, Diagnostic> {
        let rows =
            self.sections.iter().enumerate().filter(|(_, s)| s.kind == kind).collect::<Vec<_>>();
        if rows.len() > 1 {
            return Err(rejected());
        }
        Ok(rows.into_iter().next())
    }
    pub(super) fn section(&self, s: &Section) -> Result<&'a [u8], Diagnostic> {
        let data = range(self.bytes, s.offset, s.size)?;
        if self.file_offset(s.address, s.size)? != s.offset {
            return Err(rejected());
        }
        Ok(data)
    }
}
