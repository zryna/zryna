use std::collections::BTreeMap;
use std::time::Instant;

use sha2::{Digest, Sha256};

use super::{Error, Expected, basename, check, header, padded};

pub(super) fn extract(
    deadline: Instant,
    data: &[u8],
    expected: &Expected,
    limit: usize,
) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    check(deadline)?;
    if data.len() > limit || !data.len().is_multiple_of(512) || expected.files.len() > 64 {
        return Err(Error::Limit);
    }
    if expected.files.keys().any(|name| !basename(name)) {
        return Err(Error::Inventory);
    }
    let directory_name = format!("{}/", expected.root.name());
    let prefix = directory_name.as_str();
    let mut files = BTreeMap::new();
    let mut directory = false;
    let mut offset = 0usize;
    let mut members = 0usize;
    loop {
        check(deadline)?;
        let end = offset.checked_add(512).ok_or(Error::Limit)?;
        let block = data.get(offset..end).ok_or(Error::Format)?;
        if block.iter().all(|byte| *byte == 0) {
            if data.len() - offset != 1024 {
                return Err(Error::Format);
            }
            check(deadline)?;
            if data[offset + 512..].iter().any(|byte| *byte != 0) {
                return Err(Error::Format);
            }
            if !directory || files.len() != expected.files.len() {
                return Err(Error::Inventory);
            }
            check(deadline)?;
            return Ok(files);
        }
        members += 1;
        if members > expected.files.len() + 1 {
            return Err(Error::Inventory);
        }
        let member = header::parse(block)?;
        offset = end;
        if member.directory {
            if directory || (member.name != directory_name && member.name != expected.root.name()) {
                return Err(Error::Inventory);
            }
            if member.metadata != expected.directory {
                return Err(Error::Identity);
            }
            directory = true;
            continue;
        }
        let name = member
            .name
            .strip_prefix(prefix)
            .filter(|name| basename(name))
            .ok_or(Error::Inventory)?;
        let identity = expected.files.get(name).ok_or(Error::Inventory)?;
        if files.contains_key(name) {
            return Err(Error::Inventory);
        }
        if member.metadata != identity.metadata || member.bytes != identity.bytes {
            return Err(Error::Identity);
        }
        let (body, next) = file_body(deadline, data, offset, member.bytes, identity.sha256)?;
        files.insert(name.to_owned(), body.to_vec());
        check(deadline)?;
        offset = next;
    }
}

pub(super) fn file_body(
    deadline: Instant,
    data: &[u8],
    offset: usize,
    bytes: usize,
    sha256: [u8; 32],
) -> Result<(&[u8], usize), Error> {
    let end = offset.checked_add(bytes).ok_or(Error::Limit)?;
    let body = data.get(offset..end).ok_or(Error::Format)?;
    let mut hash = Sha256::new();
    for block in body.chunks(512) {
        check(deadline)?;
        hash.update(block);
    }
    if <[u8; 32]>::from(hash.finalize()) != sha256 {
        return Err(Error::Identity);
    }
    let next = offset.checked_add(padded(bytes)?).ok_or(Error::Limit)?;
    for block in data.get(end..next).ok_or(Error::Format)?.chunks(512) {
        check(deadline)?;
        if block.iter().any(|byte| *byte != 0) {
            return Err(Error::Format);
        }
    }
    check(deadline)?;
    Ok((body, next))
}
