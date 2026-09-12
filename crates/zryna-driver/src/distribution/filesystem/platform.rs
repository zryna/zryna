use std::{
    fs, io,
    path::{Component, Path},
};

use cap_fs_ext::DirExt as _;
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;

pub(super) fn directory_identity(directory: &Dir) -> io::Result<Handle> {
    directory.try_clone().map(Dir::into_std_file).and_then(Handle::from_file)
}

pub(super) fn child(parent: &Dir, name: &std::ffi::OsStr) -> io::Result<Dir> {
    let directory = parent.open_dir_nofollow(name)?;
    let metadata = directory_identity(&directory)?.as_file().metadata()?;
    if !metadata.is_dir() || link_or_reparse(&metadata) {
        return Err(io::Error::other("unsafe installation directory"));
    }
    Ok(directory)
}

#[cfg(windows)]
pub(in crate::distribution) fn absolute(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
    use std::path::{PathBuf, Prefix};
    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => return Err(io::Error::other("unsupported installation root")),
        },
        _ => return Err(io::Error::other("unsupported installation root")),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(io::Error::other("unsupported installation root"));
    }
    let anchor = Dir::open_ambient_dir(
        PathBuf::from(format!("{}:\\", char::from(drive))),
        ambient_authority(),
    )?;
    components_from(anchor, components)
}

#[cfg(unix)]
pub(in crate::distribution) fn absolute(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
    if !path.is_absolute() {
        return Err(io::Error::other("installation root is not absolute"));
    }
    components_from(Dir::open_ambient_dir("/", ambient_authority())?, path.components())
}

fn components_from<'a>(
    anchor: Dir,
    components: impl Iterator<Item = Component<'a>>,
) -> io::Result<(Vec<Dir>, Dir)> {
    let mut anchors = vec![anchor];
    for component in components {
        match component {
            Component::Normal(name) => {
                let parent = anchors.last().ok_or_else(|| io::Error::other("missing anchor"))?;
                anchors.push(child(parent, name)?);
            }
            Component::RootDir => {}
            _ => return Err(io::Error::other("noncanonical installation path")),
        }
    }
    let root = anchors.last().ok_or_else(|| io::Error::other("missing root"))?.try_clone()?;
    Ok((anchors, root))
}

#[cfg(windows)]
pub(super) fn link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(unix)]
pub(super) fn link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

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

#[cfg(unix)]
pub(super) fn same_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.size() == right.size()
        && left.mode() == right.mode()
        && left.nlink() == right.nlink()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
pub(super) fn same_state(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}
