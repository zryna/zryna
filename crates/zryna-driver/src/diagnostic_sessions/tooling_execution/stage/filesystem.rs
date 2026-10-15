//! No-follow stage handles and platform identity/state checks.

use super::stage_changed;
#[cfg(target_os = "linux")]
use super::{ROOT, RetainedDirectory};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File};
use same_file::Handle;
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::{collections::BTreeMap, path::PathBuf};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use zryna_diagnostics::Diagnostic;

pub(super) fn validate_root_path(path: &Path) -> Result<Dir, Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|_| stage_changed())?;
    if !metadata.is_dir() || metadata_is_link_or_reparse(&metadata) {
        return Err(stage_changed());
    }
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).map_err(|_| stage_changed())
}

pub(super) fn open_regular(parent: &Dir, name: &str) -> Result<Handle, Diagnostic> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    configure_read(&mut options);
    parent
        .open_with(name, &options)
        .map(File::into_std)
        .and_then(Handle::from_file)
        .map_err(|_| stage_changed())
}

pub(super) fn hash_handle(handle: &mut Handle) -> Result<[u8; 32], Diagnostic> {
    handle.as_file_mut().seek(SeekFrom::Start(0)).map_err(|_| stage_changed())?;
    let mut digest = Sha256::new();
    let mut limited = handle.as_file_mut().take(16 * 1_024 * 1_024 + 1);
    std::io::copy(&mut limited, &mut digest).map_err(|_| stage_changed())?;
    if limited.limit() == 0 {
        return Err(stage_changed());
    }
    Ok(digest.finalize().into())
}

pub(super) fn directory_identity(directory: &Dir) -> Result<Handle, Diagnostic> {
    directory
        .try_clone()
        .map(Dir::into_std_file)
        .and_then(Handle::from_file)
        .map_err(|_| stage_changed())
}

#[cfg(target_os = "linux")]
pub(super) fn capability_root(
    directories: &BTreeMap<&'static str, RetainedDirectory>,
) -> Result<PathBuf, Diagnostic> {
    use std::os::fd::AsRawFd as _;
    let root = directories.get(ROOT).ok_or_else(stage_changed)?;
    Ok(PathBuf::from(format!("/proc/{}/fd/{}", std::process::id(), root.directory()?.as_raw_fd())))
}

#[cfg(unix)]
pub(super) fn configure_create(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.mode(0o600);
}

#[cfg(windows)]
pub(super) fn configure_create(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.share_mode(1 | 4);
}

#[cfg(not(any(unix, windows)))]
pub(super) fn configure_create(_options: &mut cap_std::fs::OpenOptions) {}

#[cfg(unix)]
pub(super) fn configure_read(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.custom_flags(libc::O_NONBLOCK);
}

#[cfg(windows)]
pub(super) fn configure_read(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.share_mode(1);
}

#[cfg(not(any(unix, windows)))]
pub(super) fn configure_read(_options: &mut cap_std::fs::OpenOptions) {}

#[cfg(unix)]
pub(super) fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
pub(super) fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
pub(super) fn same_file_state(_left: &fs::Metadata, _right: &fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
pub(super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
pub(super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
