use std::collections::BTreeMap;
use std::time::Instant;

use super::{Error, basename, check, header, padded};

pub(super) fn input(
    deadline: Instant,
    files: &BTreeMap<String, Vec<u8>>,
    limit: usize,
) -> Result<Vec<u8>, Error> {
    check(deadline)?;
    if files.len() > 64 {
        return Err(Error::Limit);
    }
    let mut size = 1024usize;
    for (name, body) in files {
        check(deadline)?;
        if !basename(name) {
            return Err(Error::Inventory);
        }
        size = size
            .checked_add(512)
            .and_then(|size| size.checked_add(padded(body.len()).ok()?))
            .ok_or(Error::Limit)?;
        if size > limit {
            return Err(Error::Limit);
        }
    }
    if size > limit {
        return Err(Error::Limit);
    }
    let mut output = Vec::new();
    for (name, body) in files {
        check(deadline)?;
        output.extend_from_slice(&header::encode(name, body.len(), false)?);
        for block in body.chunks(512) {
            check(deadline)?;
            output.extend_from_slice(block);
        }
        let padding = padded(body.len())? - body.len();
        output.resize(output.len() + padding, 0);
    }
    check(deadline)?;
    output.resize(size, 0);
    check(deadline)?;
    Ok(output)
}

#[cfg(test)]
mod tests;
