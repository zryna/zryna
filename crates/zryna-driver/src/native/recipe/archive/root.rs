use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::{Error, RootEntry, RootKind, check, header, parse};

fn absolute(name: &str) -> bool {
    name.starts_with('/')
        && name.len() < 100
        && name.is_ascii()
        && name.bytes().all(|byte| (0x21..=0x7e).contains(&byte) && byte != b'\\')
        && name[1..].split('/').all(|part| !part.is_empty() && !matches!(part, "." | ".."))
}

pub(super) fn extract(
    deadline: Instant,
    data: &[u8],
    expected: &BTreeMap<String, RootEntry>,
    limit: usize,
) -> Result<(), Error> {
    check(deadline)?;
    if data.len() > limit || !data.len().is_multiple_of(512) || expected.len() > 128 {
        return Err(Error::Limit);
    }
    if !matches!(expected.get("").map(|entry| &entry.kind), Some(RootKind::Directory)) {
        return Err(Error::Inventory);
    }
    for (name, entry) in expected {
        check(deadline)?;
        if (!name.is_empty() && !absolute(name))
            || (name.is_empty() && !matches!(entry.kind, RootKind::Directory))
        {
            return Err(Error::Inventory);
        }
        if let RootKind::Symlink { target } = &entry.kind
            && !absolute(target)
        {
            return Err(Error::Inventory);
        }
    }
    let mut seen = BTreeSet::new();
    let mut offset = 0usize;
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
            if seen.len() != expected.len() {
                return Err(Error::Inventory);
            }
            check(deadline)?;
            return Ok(());
        }
        if seen.len() == expected.len() {
            return Err(Error::Inventory);
        }
        let member = header::parse_root(block)?;
        let name = if member.directory {
            member.name.strip_suffix('/').ok_or(Error::Inventory)?
        } else {
            member.name.as_str()
        };
        if !name.is_empty() && !absolute(name) {
            return Err(Error::Inventory);
        }
        let entry = expected.get(name).ok_or(Error::Inventory)?;
        if !seen.insert(name.to_owned()) {
            return Err(Error::Inventory);
        }
        if member.metadata != entry.metadata {
            return Err(Error::Identity);
        }
        offset = end;
        match &entry.kind {
            RootKind::Directory => {
                if !member.directory || member.link.is_some() {
                    return Err(Error::Identity);
                }
            }
            RootKind::Symlink { target } => {
                if member.directory || member.link.as_ref() != Some(target) {
                    return Err(Error::Identity);
                }
            }
            RootKind::File { bytes, sha256 } => {
                if member.directory || member.link.is_some() || member.bytes != *bytes {
                    return Err(Error::Identity);
                }
                let (_, next) = parse::file_body(deadline, data, offset, *bytes, *sha256)?;
                check(deadline)?;
                offset = next;
            }
        }
    }
}

#[cfg(test)]
mod tests;
