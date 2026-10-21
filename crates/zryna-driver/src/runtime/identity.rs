//! Retained, no-follow executable identity and platform invocation helpers.
use std::{
    io,
    path::{Path, PathBuf},
};

use same_file::Handle;
use zryna_diagnostics::Diagnostic;

use super::runtime_error;

pub(crate) fn open_runtime_identity(
    path: &Path,
) -> Result<(Handle, std::fs::Metadata), Diagnostic> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| {
        runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime path could not be inspected",
            "pass the absolute path of the documented Node.js 22.22.1 executable",
        )
    })?;
    if !metadata.is_file() || metadata_is_link_or_reparse(&metadata) {
        return Err(runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime path is not a real regular file",
            "pass a direct executable path without symbolic links or reparse points",
        ));
    }
    let file = open_regular_no_follow(path).map_err(|_| {
        runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime could not be opened safely",
            "pass a direct executable path without symbolic links or reparse points",
        )
    })?;
    let handle = Handle::from_file(file).map_err(|_| {
        runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime identity could not be established",
            "pass a stable direct executable path and retry",
        )
    })?;
    let opened = handle.as_file().metadata().map_err(|_| {
        runtime_error(
            "ZRYNA-R3001",
            "Node.js runtime metadata could not be read",
            "pass a stable direct executable path and retry",
        )
    })?;
    if !opened.is_file() || metadata_is_link_or_reparse(&opened) {
        return Err(runtime_error(
            "ZRYNA-R3001",
            "opened Node.js runtime is not a real regular file",
            "pass a direct executable path without symbolic links or reparse points",
        ));
    }
    Ok((handle, opened))
}

#[cfg(target_os = "linux")]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn stable_invocation_path(
    _path: &Path,
    identity: &Handle,
) -> Result<PathBuf, Diagnostic> {
    use std::os::fd::AsRawFd;

    Ok(PathBuf::from(format!("/proc/self/fd/{}", identity.as_file().as_raw_fd())))
}

#[cfg(windows)]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn stable_invocation_path(
    path: &Path,
    _identity: &Handle,
) -> Result<PathBuf, Diagnostic> {
    Ok(path.to_path_buf())
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn stable_invocation_path(
    _path: &Path,
    _identity: &Handle,
) -> Result<PathBuf, Diagnostic> {
    Err(runtime_error(
        "ZRYNA-R3001",
        "sealed Node.js execution is unsupported on this platform",
        "run this command on documented Linux or Windows hosts",
    ))
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = std::fs::OpenOptions::new();
    options.read(true).custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    options.open(path)
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).share_mode(FILE_SHARE_READ);
    options.open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_regular_no_follow(_path: &Path) -> io::Result<std::fs::File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no fail-closed no-follow strategy exists for this platform",
    ))
}

#[cfg(unix)]
pub(crate) fn same_file_state(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
pub(crate) fn same_file_state(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn same_file_state(_left: &std::fs::Metadata, _right: &std::fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
fn metadata_is_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
