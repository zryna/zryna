use std::io::{BufReader, Read};
use std::time::Instant;

use super::headers::{Framing, Headers};
use super::types::{Error, Response, io_error, remaining};

struct Input<R> {
    input: BufReader<R>,
    deadline: Instant,
}

impl<R: Read> Input<R> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        loop {
            remaining(self.deadline)?;
            let result = self.input.read(bytes);
            remaining(self.deadline)?;
            match result {
                Ok(count) => return Ok(count),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return Err(io_error(self.deadline)),
            }
        }
    }

    fn byte(&mut self) -> Result<Option<u8>, Error> {
        let mut byte = [0];
        Ok((self.read(&mut byte)? != 0).then_some(byte[0]))
    }

    fn line(&mut self, budget: &mut usize) -> Result<Vec<u8>, Error> {
        let mut line = Vec::new();
        loop {
            if *budget == 0 {
                return Err(Error::Limit);
            }
            *budget -= 1;
            let byte = self.byte()?.ok_or(Error::Http)?;
            if byte == b'\r' {
                if *budget == 0 {
                    return Err(Error::Limit);
                }
                *budget -= 1;
                if self.byte()? != Some(b'\n') {
                    return Err(Error::Http);
                }
                return Ok(line);
            }
            if byte == b'\n' {
                return Err(Error::Http);
            }
            line.push(byte);
        }
    }

    fn exact(&mut self, body: &mut Vec<u8>, count: usize, limit: usize) -> Result<(), Error> {
        let end = body.len().checked_add(count).filter(|end| *end <= limit).ok_or(Error::Limit)?;
        let mut buffer = [0; 8192];
        while body.len() < end {
            let wanted = (end - body.len()).min(buffer.len());
            let count = self.read(&mut buffer[..wanted])?;
            if count == 0 {
                return Err(Error::Http);
            }
            body.extend_from_slice(&buffer[..count]);
            remaining(self.deadline)?;
        }
        Ok(())
    }

    fn eof(&mut self) -> Result<(), Error> {
        if self.byte()?.is_some() {
            return Err(Error::Http);
        }
        Ok(())
    }
}

pub(super) fn response<R: Read>(
    input: R,
    deadline: Instant,
    limit: usize,
) -> Result<Response, Error> {
    let mut input = Input { input: BufReader::with_capacity(8192, input), deadline };
    let mut budget = 16 * 1024;
    let status = super::headers::status(&input.line(&mut budget)?)?;
    let mut headers = Headers::new();
    loop {
        let line = input.line(&mut budget)?;
        if line.is_empty() {
            break;
        }
        headers.field(&line)?;
    }
    let framing = headers.framing()?;
    let mut body = Vec::new();
    match framing {
        Framing::Length(count) => {
            input.exact(&mut body, count, limit)?;
        }
        Framing::Chunked => chunks(&mut input, &mut body, limit)?,
        Framing::Close => {
            let mut buffer = [0; 8192];
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                if body.len().checked_add(count).is_none_or(|end| end > limit) {
                    return Err(Error::Limit);
                }
                body.extend_from_slice(&buffer[..count]);
            }
        }
    }
    if matches!(status, 204 | 304) && !body.is_empty() {
        return Err(Error::Http);
    }
    input.eof()?;
    remaining(deadline)?;
    Ok(Response { status, body })
}

fn chunks<R: Read>(input: &mut Input<R>, body: &mut Vec<u8>, limit: usize) -> Result<(), Error> {
    for _ in 0..4096 {
        let line = input.line(&mut 128)?;
        if line.is_empty() || !line.iter().all(u8::is_ascii_hexdigit) {
            return Err(Error::Http);
        }
        let count = line.iter().try_fold(0usize, |size, byte| {
            let digit = char::from(*byte).to_digit(16).ok_or(Error::Http)?;
            size.checked_mul(16)
                .and_then(|size| size.checked_add(digit as usize))
                .ok_or(Error::Http)
        })?;
        if count == 0 {
            // No trailer fields, declarations or chunk extensions are admitted.
            if !input.line(&mut 2)?.is_empty() {
                return Err(Error::Http);
            }
            return Ok(());
        }
        input.exact(body, count, limit)?;
        if input.byte()? != Some(b'\r') || input.byte()? != Some(b'\n') {
            return Err(Error::Http);
        }
    }
    Err(Error::Limit)
}
