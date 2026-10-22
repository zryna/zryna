//! Bounded no-follow source capture and cross-platform file identity checks.

use super::{CapturedFile, execution_error};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Component, Path},
};
use zryna_diagnostics::Diagnostic;

pub(in super::super) fn capture_file(
    root: &Dir,
    components: &[&str],
    limit: usize,
) -> Result<CapturedFile, Diagnostic> {
    let (name, directories) =
        components.split_last().ok_or_else(|| execution_error("tooling source path is empty"))?;
    let parent = open_dir(root, directories)?;
    let mut opened = open_regular(&parent, name)?;
    let metadata = opened.as_file().metadata().map_err(|_| source_changed())?;
    if !metadata.is_file() || metadata_is_link_or_reparse(&metadata) {
        return Err(source_changed());
    }
    let bytes = bounded_read(&mut opened, limit)?;
    let reopened = open_regular(&parent, name)?;
    let current = reopened.as_file().metadata().map_err(|_| source_changed())?;
    let held = opened.as_file().metadata().map_err(|_| source_changed())?;
    if reopened != opened
        || !same_file_state(&metadata, &current)
        || !same_file_state(&metadata, &held)
    {
        return Err(source_changed());
    }
    Ok(CapturedFile { sha256: Sha256::digest(&bytes).into(), bytes })
}

pub(in super::super) fn open_dir(root: &Dir, components: &[&str]) -> Result<Dir, Diagnostic> {
    let mut current = root.try_clone().map_err(|_| source_changed())?;
    for component in components {
        let child = current.open_dir_nofollow(component).map_err(|_| source_changed())?;
        let reopened = current.open_dir_nofollow(component).map_err(|_| source_changed())?;
        if directory_identity(&child)? != directory_identity(&reopened)? {
            return Err(source_changed());
        }
        current = child;
    }
    Ok(current)
}

pub(in super::super) fn open_regular(parent: &Dir, name: &str) -> Result<Handle, Diagnostic> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    configure_open(&mut options);
    parent
        .open_with(OsStr::new(name), &options)
        .map(cap_std::fs::File::into_std)
        .and_then(Handle::from_file)
        .map_err(|_| source_changed())
}

fn bounded_read(handle: &mut Handle, limit: usize) -> Result<Vec<u8>, Diagnostic> {
    let length = handle.as_file().metadata().map_err(|_| source_changed())?.len();
    if length > u64::try_from(limit).unwrap_or(u64::MAX) {
        return Err(execution_error("tooling executable source exceeds its byte limit"));
    }
    handle.as_file_mut().seek(SeekFrom::Start(0)).map_err(|_| source_changed())?;
    let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(limit).min(limit));
    handle
        .as_file_mut()
        .take(u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| source_changed())?;
    if bytes.len() > limit {
        Err(execution_error("tooling executable source exceeds its byte limit"))
    } else {
        Ok(bytes)
    }
}

pub(in super::super) fn capture_absolute(path: &Path) -> Result<Dir, Diagnostic> {
    #[cfg(unix)]
    let (mut current, mut components) = (
        Dir::open_ambient_dir(Path::new("/"), ambient_authority()).map_err(|_| source_changed())?,
        path.components(),
    );
    #[cfg(windows)]
    let (mut current, mut components) = windows_anchor(path)?;
    #[cfg(not(any(unix, windows)))]
    return Err(execution_error("tooling execution is unsupported on this platform"));
    for component in &mut components {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(name) => {
                let child = current.open_dir_nofollow(name).map_err(|_| source_changed())?;
                let reopened = current.open_dir_nofollow(name).map_err(|_| source_changed())?;
                if directory_identity(&child)? != directory_identity(&reopened)? {
                    return Err(source_changed());
                }
                current = child;
            }
            Component::Prefix(_) if cfg!(windows) => {}
            Component::ParentDir | Component::Prefix(_) => return Err(source_changed()),
        }
    }
    Ok(current)
}

#[cfg(windows)]
fn windows_anchor(path: &Path) -> Result<(Dir, std::path::Components<'_>), Diagnostic> {
    use std::path::{PathBuf, Prefix};
    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => return Err(source_changed()),
        },
        _ => return Err(source_changed()),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(source_changed());
    }
    let anchor = Dir::open_ambient_dir(
        PathBuf::from(format!("{}:\\", char::from(drive))),
        ambient_authority(),
    )
    .map_err(|_| source_changed())?;
    Ok((anchor, components))
}

pub(in super::super) fn directory_identity(directory: &Dir) -> Result<Handle, Diagnostic> {
    directory
        .try_clone()
        .map(Dir::into_std_file)
        .and_then(Handle::from_file)
        .map_err(|_| source_changed())
}

pub(in super::super) fn source_changed() -> Diagnostic {
    execution_error("tooling executable source is unavailable, linked, or changed during capture")
}

#[cfg(unix)]
fn configure_open(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    options.custom_flags(libc::O_NONBLOCK);
}

#[cfg(windows)]
fn configure_open(options: &mut cap_std::fs::OpenOptions) {
    use cap_std::fs::OpenOptionsExt as _;
    const FILE_SHARE_READ: u32 = 1;
    options.share_mode(FILE_SHARE_READ);
}

#[cfg(not(any(unix, windows)))]
fn configure_open(_options: &mut cap_std::fs::OpenOptions) {}

#[cfg(unix)]
fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.size() == right.size()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
fn same_file_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
fn same_file_state(_left: &fs::Metadata, _right: &fs::Metadata) -> bool {
    false
}

#[cfg(windows)]
pub(in super::super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x0400 != 0
}

#[cfg(windows)]
pub(in super::super) fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    use cap_std::fs::MetadataExt as _;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
pub(in super::super) fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(not(windows))]
pub(in super::super) fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
