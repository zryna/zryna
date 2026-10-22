use super::{Operation, Result, archive_header as header, reject};
use std::{collections::BTreeMap, time::Instant};

pub(super) const LIMIT: usize = 2 * 1024 * 1024 + 65_536;

fn check(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(reject("PLAYGROUND-HOST-DEADLINE", "archive observation deadline exceeded"))
    } else {
        Ok(())
    }
}

fn padded(bytes: usize) -> Result<usize> {
    bytes
        .checked_add(511)
        .map(|bytes| bytes / 512 * 512)
        .ok_or_else(|| reject("PLAYGROUND-HOST-ARCHIVE", "archive integer bound exceeded"))
}

pub(super) fn input(request: &[u8], deadline: Instant) -> Result<Vec<u8>> {
    check(deadline)?;
    let size = padded(request.len())?
        .checked_add(1536)
        .filter(|size| *size <= LIMIT)
        .ok_or_else(|| reject("PLAYGROUND-HOST-ARCHIVE", "request archive exceeds finite bound"))?;
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(&header::encode("request", request.len(), false, 0, 0, 0o444)?);
    bytes.extend_from_slice(request);
    bytes.resize(size, 0);
    check(deadline)?;
    Ok(bytes)
}

pub(super) fn verify_input(data: &[u8], request: &[u8], deadline: Instant) -> Result<()> {
    let files =
        extract(data, "input", 0, 0o555, &[("request", request.len(), 0, 0o444)], deadline)?;
    if files.get("request").map(Vec::as_slice) != Some(request) {
        return Err(reject("PLAYGROUND-HOST-ARCHIVE", "retained input bytes differ after staging"));
    }
    Ok(())
}

pub(super) fn output(
    data: &[u8],
    operation: Operation,
    deadline: Instant,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut files = extract(
        data,
        "output",
        65534,
        0o700,
        &[("stdout", operation.stdout_limit(), 65534, 0o600), ("stderr", 4096, 65534, 0o600)],
        deadline,
    )?;
    let stdout = files
        .remove("stdout")
        .ok_or_else(|| reject("PLAYGROUND-HOST-ARCHIVE", "stdout member absent"))?;
    let stderr = files
        .remove("stderr")
        .ok_or_else(|| reject("PLAYGROUND-HOST-ARCHIVE", "stderr member absent"))?;
    Ok((stdout, stderr))
}

fn extract(
    data: &[u8],
    root: &str,
    uid: u32,
    mode: u32,
    expected: &[(&str, usize, u32, u32)],
    deadline: Instant,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let bad = || {
        reject("PLAYGROUND-HOST-ARCHIVE", "complete fixed archive inventory or metadata differs")
    };
    check(deadline)?;
    if data.len() > LIMIT || !data.len().is_multiple_of(512) {
        return Err(bad());
    }
    let mut files = BTreeMap::new();
    let mut offset = 0usize;
    let mut directory = false;
    let prefix = format!("{root}/");
    let mut members = 0usize;
    loop {
        check(deadline)?;
        let end = offset.checked_add(512).ok_or_else(bad)?;
        let block = data.get(offset..end).ok_or_else(bad)?;
        if block.iter().all(|byte| *byte == 0) {
            if data.len() - offset != 1024
                || data[offset + 512..].iter().any(|byte| *byte != 0)
                || !directory
                || files.len() != expected.len()
            {
                return Err(bad());
            }
            return Ok(files);
        }
        members += 1;
        if members > expected.len() + 1 {
            return Err(bad());
        }
        let member = header::parse(block)?;
        offset = end;
        if member.directory {
            if directory
                || member.name != prefix && member.name != root
                || member.uid != uid
                || member.gid != uid
                || member.mode != mode
            {
                return Err(bad());
            }
            directory = true;
            continue;
        }
        let name = member.name.strip_prefix(&prefix).ok_or_else(bad)?;
        let (_, limit, file_uid, file_mode) =
            expected.iter().find(|(entry, ..)| *entry == name).ok_or_else(bad)?;
        if member.bytes > *limit
            || member.uid != *file_uid
            || member.gid != *file_uid
            || member.mode != *file_mode
            || files.contains_key(name)
        {
            return Err(bad());
        }
        let body_end = offset.checked_add(member.bytes).ok_or_else(bad)?;
        let next = offset.checked_add(padded(member.bytes)?).ok_or_else(bad)?;
        let body = data.get(offset..body_end).ok_or_else(bad)?;
        if data.get(body_end..next).ok_or_else(bad)?.iter().any(|byte| *byte != 0) {
            return Err(bad());
        }
        files.insert(name.to_owned(), body.to_vec());
        offset = next;
    }
}
