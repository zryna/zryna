use super::{Result, reject};

pub(super) struct Header {
    pub name: String,
    pub bytes: usize,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub directory: bool,
}

fn bad() -> super::PlaygroundDaemonError {
    reject("PLAYGROUND-HOST-ARCHIVE", "closed ordinary USTAR archive required")
}

fn number(field: &[u8]) -> Result<u64> {
    let field = field.trim_ascii();
    let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    if end == 0 || field[end..].iter().any(|byte| *byte != 0) {
        return Err(bad());
    }
    field[..end].iter().try_fold(0u64, |value, byte| {
        if !(b'0'..=b'7').contains(byte) {
            return Err(bad());
        }
        value
            .checked_mul(8)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or_else(bad)
    })
}

pub(super) fn parse(block: &[u8]) -> Result<Header> {
    if block.len() != 512
        || &block[257..265] != b"ustar\x0000"
        || block[265..329].iter().any(|byte| *byte != 0)
        || block[345..].iter().any(|byte| *byte != 0)
        || block[157..257].iter().any(|byte| *byte != 0)
    {
        return Err(bad());
    }
    let checksum: u64 = block
        .iter()
        .enumerate()
        .map(|(i, byte)| if (148..156).contains(&i) { 32 } else { u64::from(*byte) })
        .sum();
    if number(&block[148..156])? != checksum
        || number(&block[329..337])? != 0
        || number(&block[337..345])? != 0
    {
        return Err(bad());
    }
    let end = block[..100].iter().position(|byte| *byte == 0).ok_or_else(bad)?;
    if block[end..100].iter().any(|byte| *byte != 0) {
        return Err(bad());
    }
    let name = std::str::from_utf8(&block[..end]).map_err(|_| bad())?;
    if !name.is_ascii() {
        return Err(bad());
    }
    let directory = match block[156] {
        b'5' => true,
        b'0' | 0 => false,
        _ => return Err(bad()),
    };
    let bytes = usize::try_from(number(&block[124..136])?).map_err(|_| bad())?;
    number(&block[136..148])?;
    if directory && bytes != 0 {
        return Err(bad());
    }
    Ok(Header {
        name: name.to_owned(),
        bytes,
        directory,
        mode: u32::try_from(number(&block[100..108])?).map_err(|_| bad())?,
        uid: u32::try_from(number(&block[108..116])?).map_err(|_| bad())?,
        gid: u32::try_from(number(&block[116..124])?).map_err(|_| bad())?,
    })
}

fn octal(field: &mut [u8], mut value: u64) -> Result<()> {
    field.fill(b'0');
    let last = field.len() - 1;
    field[last] = 0;
    for byte in field[..last].iter_mut().rev() {
        *byte = b'0' + u8::try_from(value & 7).map_err(|_| bad())?;
        value >>= 3;
    }
    if value != 0 {
        return Err(bad());
    }
    Ok(())
}

pub(super) fn encode(
    name: &str,
    bytes: usize,
    directory: bool,
    uid: u32,
    gid: u32,
    mode: u32,
) -> Result<[u8; 512]> {
    if name.len() >= 100 {
        return Err(bad());
    }
    let mut block = [0; 512];
    block[..name.len()].copy_from_slice(name.as_bytes());
    for (range, value) in [
        (100..108, u64::from(mode)),
        (108..116, u64::from(uid)),
        (116..124, u64::from(gid)),
        (124..136, u64::try_from(bytes).map_err(|_| bad())?),
        (136..148, 0),
        (329..337, 0),
        (337..345, 0),
    ] {
        octal(&mut block[range], value)?;
    }
    block[148..156].fill(b' ');
    block[156] = if directory { b'5' } else { b'0' };
    block[257..265].copy_from_slice(b"ustar\x0000");
    let checksum = block.iter().map(|byte| u64::from(*byte)).sum();
    octal(&mut block[148..155], checksum)?;
    block[155] = b' ';
    Ok(block)
}
