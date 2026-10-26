//! Bound before allocation/read, retain exact inode metadata, and detect one extra byte.

use super::super::{ToolFileIdentity, regular_file_identity, tool_identity_from_metadata};
use std::{fs, io::Read as _, os::unix::fs::OpenOptionsExt as _, path::Path};
use zryna_diagnostics::Diagnostic;

pub(super) fn read(path: &Path, limit: usize) -> Result<(ToolFileIdentity, Vec<u8>), Diagnostic> {
    let before = regular_file_identity(path).map_err(|()| super::staging_error())?;
    let length = usize::try_from(before.length).map_err(|_| super::staging_error())?;
    if length == 0 || length > limit {
        return Err(super::staging_error());
    }
    let capacity = length.checked_add(1).ok_or_else(super::staging_error)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| super::staging_error())?;
    let opened = tool_identity_from_metadata(&file.metadata().map_err(|_| super::staging_error())?)
        .map_err(|()| super::staging_error())?;
    if before != opened {
        return Err(super::staging_error());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| super::staging_error())?;
    file.take(u64::try_from(capacity).map_err(|_| super::staging_error())?)
        .read_to_end(&mut bytes)
        .map_err(|_| super::staging_error())?;
    let after = regular_file_identity(path).map_err(|()| super::staging_error())?;
    if before != after || bytes.len() != length {
        return Err(super::staging_error());
    }
    Ok((before, bytes))
}
