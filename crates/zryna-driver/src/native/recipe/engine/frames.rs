use std::time::Instant;

use super::types::{Error, Streams, remaining};

pub(super) fn parse(deadline: Instant, mut bytes: &[u8]) -> Result<Streams, Error> {
    let mut streams = Streams::default();
    let mut frames = 0;
    remaining(deadline)?;
    while !bytes.is_empty() {
        remaining(deadline)?;
        frames += 1;
        if frames > 4096 {
            return Err(Error::Limit);
        }
        let header = bytes.get(..8).ok_or(Error::Frames)?;
        if header[1..4] != [0, 0, 0] {
            return Err(Error::Frames);
        }
        let length = u32::from_be_bytes(header[4..8].try_into().map_err(|_| Error::Frames)?);
        let length = usize::try_from(length).map_err(|_| Error::Limit)?;
        let stream = match header[0] {
            1 => &mut streams.stdout,
            2 => &mut streams.stderr,
            _ => return Err(Error::Frames),
        };
        if stream.len().checked_add(length).is_none_or(|end| end > 65536) {
            return Err(Error::Limit);
        }
        let end = 8usize.checked_add(length).ok_or(Error::Limit)?;
        let payload = bytes.get(8..end).ok_or(Error::Frames)?;
        stream.extend_from_slice(payload);
        bytes = &bytes[end..];
    }
    remaining(deadline)?;
    Ok(streams)
}
