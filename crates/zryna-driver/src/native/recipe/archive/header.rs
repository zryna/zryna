use super::{Error, Metadata};

pub(super) struct Header {
    pub(super) name: String,
    pub(super) metadata: Metadata,
    pub(super) bytes: usize,
    pub(super) directory: bool,
    pub(super) link: Option<String>,
}

fn number(field: &[u8]) -> Result<u64, Error> {
    let field = field.trim_ascii();
    let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0) || end == 0 {
        return Err(Error::Format);
    }
    field[..end].iter().try_fold(0u64, |value, byte| {
        if !(b'0'..=b'7').contains(byte) {
            return Err(Error::Format);
        }
        value
            .checked_mul(8)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or(Error::Format)
    })
}

pub(super) fn parse(block: &[u8]) -> Result<Header, Error> {
    parse_common(block, false)
}

pub(super) fn parse_root(block: &[u8]) -> Result<Header, Error> {
    parse_common(block, true)
}

fn text(field: &[u8]) -> Result<String, Error> {
    let end = field.iter().position(|byte| *byte == 0).ok_or(Error::Format)?;
    if field[end..].iter().any(|byte| *byte != 0) {
        return Err(Error::Format);
    }
    let value = std::str::from_utf8(&field[..end]).map_err(|_| Error::Format)?;
    if !value.is_ascii() {
        return Err(Error::Format);
    }
    Ok(value.to_owned())
}

fn parse_common(block: &[u8], links: bool) -> Result<Header, Error> {
    if block.len() != 512
        || &block[257..265] != b"ustar\x0000"
        || block[265..329].iter().any(|byte| *byte != 0)
        || block[345..].iter().any(|byte| *byte != 0)
    {
        return Err(Error::Format);
    }
    let checksum = block
        .iter()
        .enumerate()
        .map(|(index, byte)| if (148..156).contains(&index) { 32u64 } else { u64::from(*byte) })
        .sum::<u64>();
    if number(&block[148..156])? != checksum
        || number(&block[329..337])? != 0
        || number(&block[337..345])? != 0
    {
        return Err(Error::Format);
    }
    let name = text(&block[..100])?;
    let (directory, link) = match block[156] {
        b'5' => (true, None),
        b'0' | 0 => (false, None),
        b'2' if links => (false, Some(text(&block[157..257])?)),
        _ => return Err(Error::Format),
    };
    if link.is_none() && block[157..257].iter().any(|byte| *byte != 0) {
        return Err(Error::Format);
    }
    let mode = u32::try_from(number(&block[100..108])?).map_err(|_| Error::Format)?;
    let uid = u32::try_from(number(&block[108..116])?).map_err(|_| Error::Format)?;
    let gid = u32::try_from(number(&block[116..124])?).map_err(|_| Error::Format)?;
    let bytes = usize::try_from(number(&block[124..136])?).map_err(|_| Error::Limit)?;
    number(&block[136..148])?;
    if mode & !0o7777 != 0 || ((directory || link.is_some()) && bytes != 0) {
        return Err(Error::Format);
    }
    Ok(Header { name, metadata: Metadata { uid, gid, mode }, bytes, directory, link })
}

fn octal(field: &mut [u8], mut value: u64) -> Result<(), Error> {
    field.fill(b'0');
    let last = field.len() - 1;
    field[last] = 0;
    for byte in field[..last].iter_mut().rev() {
        *byte = b'0' + u8::try_from(value & 7).map_err(|_| Error::Format)?;
        value >>= 3;
    }
    if value != 0 {
        return Err(Error::Limit);
    }
    Ok(())
}

pub(super) fn encode(name: &str, bytes: usize, directory: bool) -> Result<[u8; 512], Error> {
    if name.len() >= 100 {
        return Err(Error::Inventory);
    }
    let mut block = [0; 512];
    block[..name.len()].copy_from_slice(name.as_bytes());
    octal(&mut block[100..108], if directory { 0o555 } else { 0o444 })?;
    octal(&mut block[108..116], 0)?;
    octal(&mut block[116..124], 0)?;
    octal(&mut block[124..136], u64::try_from(bytes).map_err(|_| Error::Limit)?)?;
    octal(&mut block[136..148], 0)?;
    block[148..156].fill(b' ');
    block[156] = if directory { b'5' } else { b'0' };
    block[257..265].copy_from_slice(b"ustar\x0000");
    octal(&mut block[329..337], 0)?;
    octal(&mut block[337..345], 0)?;
    let checksum = block.iter().map(|byte| u64::from(*byte)).sum();
    octal(&mut block[148..155], checksum)?;
    block[155] = b' ';
    Ok(block)
}
