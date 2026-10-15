use super::types::Error;

pub(super) enum Framing {
    Length(usize),
    Chunked,
    Close,
}

pub(super) struct Headers {
    length: Option<usize>,
    chunked: bool,
    fields: usize,
}

impl Headers {
    pub(super) fn new() -> Self {
        Self { length: None, chunked: false, fields: 0 }
    }

    pub(super) fn field(&mut self, line: &[u8]) -> Result<(), Error> {
        self.fields += 1;
        if self.fields > 64 {
            return Err(Error::Limit);
        }
        let colon = line.iter().position(|byte| *byte == b':').ok_or(Error::Http)?;
        let (name, value) = (&line[..colon], &line[colon + 1..]);
        if name.is_empty()
            || !name
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(byte))
            || !value.iter().all(|byte| (0x20..=0x7e).contains(byte))
        {
            return Err(Error::Http);
        }
        let value = value.trim_ascii();
        if name.eq_ignore_ascii_case(b"content-length") {
            if self.length.is_some() || value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
                return Err(Error::Http);
            }
            self.length = Some(decimal(value)?);
        } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
            if self.chunked || !value.eq_ignore_ascii_case(b"chunked") {
                return Err(Error::Http);
            }
            self.chunked = true;
        } else if name.eq_ignore_ascii_case(b"connection") {
            if !value.eq_ignore_ascii_case(b"close") {
                return Err(Error::Http);
            }
        } else if [b"trailer".as_slice(), b"upgrade", b"proxy-connection", b"content-encoding"]
            .iter()
            .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
        {
            return Err(Error::Http);
        }
        Ok(())
    }

    pub(super) fn framing(self) -> Result<Framing, Error> {
        match (self.length, self.chunked) {
            (Some(_), true) => Err(Error::Http),
            (Some(length), false) => Ok(Framing::Length(length)),
            (None, true) => Ok(Framing::Chunked),
            (None, false) => Ok(Framing::Close),
        }
    }
}

fn decimal(bytes: &[u8]) -> Result<usize, Error> {
    bytes.iter().try_fold(0usize, |value, byte| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(usize::from(byte - b'0')))
            .ok_or(Error::Http)
    })
}

pub(super) fn status(line: &[u8]) -> Result<u16, Error> {
    if line.len() < 13
        || &line[..9] != b"HTTP/1.1 "
        || !line[9..12].iter().all(u8::is_ascii_digit)
        || line[12] != b' '
        || !line.iter().all(|byte| (0x20..=0x7e).contains(byte))
    {
        return Err(Error::Http);
    }
    let status = u16::from(line[9] - b'0') * 100
        + u16::from(line[10] - b'0') * 10
        + u16::from(line[11] - b'0');
    if !(200..=599).contains(&status) {
        return Err(Error::Http);
    }
    Ok(status)
}
