//! Existing create-only transaction filesystem primitives; policy/cfg/order unchanged.
#[cfg(not(unix))]
use super::metadata_is_link_or_reparse;
use super::{CommandFailure, transaction_error};
use cap_std::fs::Dir;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::Path,
};

pub(super) fn read_directory_names(path: &Path) -> Result<BTreeSet<String>, CommandFailure> {
    fs::read_dir(path)
        .map_err(|_| transaction_error("private stage directory could not be enumerated"))?
        .map(|entry| {
            entry
                .map_err(|_| transaction_error("private stage entry could not be inspected"))?
                .file_name()
                .into_string()
                .map_err(|_| transaction_error("private stage entry name is not portable UTF-8"))
        })
        .collect()
}

#[cfg(unix)]
pub(super) fn is_executable_file(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 == 0o111
}

#[cfg(not(unix))]
pub(super) fn is_executable_file(_metadata: &fs::Metadata) -> bool {
    true
}

pub(super) fn create_owned_directory(path: &Path) -> Result<(), CommandFailure> {
    create_private_directory(path)
        .map_err(|_| transaction_error("could not create secure target stage directory"))
}

#[cfg(unix)]
pub(super) fn create_private_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
pub(super) fn create_private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

pub(super) fn write_complete_file(path: &Path, bytes: &[u8]) -> Result<(), CommandFailure> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| transaction_error("could not create staged file"))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|_| transaction_error("could not write and synchronize staged file"))
}

#[cfg(unix)]
pub(super) fn prepare_executable_file(path: &Path) -> Result<(), CommandFailure> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .and_then(|()| fs::File::open(path)?.sync_all())
        .map_err(|_| transaction_error("could not apply and synchronize executable mode"))
}

#[cfg(not(unix))]
pub(super) fn prepare_executable_file(path: &Path) -> Result<(), CommandFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| transaction_error("could not revalidate executable artifact"))?;
    if metadata.is_file() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(transaction_error("executable artifact is not a real regular file"))
    }
}

#[cfg(unix)]
pub(super) fn sync_directory(path: &Path) -> Result<(), CommandFailure> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| transaction_error("could not synchronize transaction directory"))
}

#[cfg(not(unix))]
pub(super) fn sync_directory(path: &Path) -> Result<(), CommandFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| transaction_error("could not revalidate transaction directory"))?;
    if metadata.is_dir() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(transaction_error("transaction path is not a real directory"))
    }
}

#[cfg(target_os = "linux")]
pub(super) fn rename_create_only(
    root: &Dir,
    source: &str,
    destination: &str,
) -> Result<(), CommandFailure> {
    use nix::fcntl::{RenameFlags, renameat2};
    use std::os::fd::AsFd;

    renameat2(root.as_fd(), source, root.as_fd(), destination, RenameFlags::RENAME_NOREPLACE)
        .map_err(|_| transaction_error("create-only bundle commit failed"))
}

#[cfg(windows)]
pub(super) fn rename_create_only(
    root: &Dir,
    source: &str,
    destination: &str,
) -> Result<(), CommandFailure> {
    root.rename(source, root, destination)
        .map_err(|_| transaction_error("create-only bundle commit failed"))
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(super) fn rename_create_only(
    _root: &Dir,
    _source: &str,
    _destination: &str,
) -> Result<(), CommandFailure> {
    Err(transaction_error("create-only bundle commit is unsupported on this platform"))
}

pub(crate) fn ensure_absent(path: &Path) -> Result<(), CommandFailure> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(transaction_error("create-only output bundle already exists")),
        Err(_) => Err(transaction_error("output bundle destination could not be inspected")),
    }
}
