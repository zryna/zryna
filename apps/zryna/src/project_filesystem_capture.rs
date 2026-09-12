use std::{
    ffi::OsStr,
    fs, io,
    path::{Component, Path},
};

use cap_fs_ext::DirExt as _;
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;

pub(super) fn directory_identity(directory: &Dir) -> io::Result<Handle> {
    directory.try_clone().map(Dir::into_std_file).and_then(Handle::from_file)
}

fn capture_child(parent: &Dir, name: &OsStr) -> io::Result<Dir> {
    let child = parent.open_dir_nofollow(name)?;
    let metadata = child.try_clone().map(Dir::into_std_file)?.metadata()?;
    if !metadata.is_dir() || metadata_is_link_or_reparse(&metadata) {
        return Err(io::Error::other("unsafe project parent component"));
    }
    Ok(child)
}

#[cfg(windows)]
pub(super) fn capture_absolute_root(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
    use std::path::{PathBuf, Prefix};

    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => return Err(io::Error::other("unsupported project parent root")),
        },
        _ => return Err(io::Error::other("unsupported project parent root")),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(io::Error::other("unsupported project parent root"));
    }
    let anchor = Dir::open_ambient_dir(
        PathBuf::from(format!("{}:\\", char::from(drive))),
        ambient_authority(),
    )?;
    capture_components(anchor, components)
}

#[cfg(unix)]
pub(super) fn capture_absolute_root(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
    capture_components(Dir::open_ambient_dir("/", ambient_authority())?, path.components())
}

fn capture_components<'a>(
    anchor: Dir,
    components: impl Iterator<Item = Component<'a>>,
) -> io::Result<(Vec<Dir>, Dir)> {
    let mut anchors = vec![anchor];
    for component in components {
        match component {
            Component::Normal(name) => {
                let parent = anchors.last().ok_or_else(|| io::Error::other("project parent"))?;
                anchors.push(capture_child(parent, name)?);
            }
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) => {
                return Err(io::Error::other("unsafe project parent"));
            }
        }
    }
    let directory =
        anchors.last().ok_or_else(|| io::Error::other("project parent"))?.try_clone()?;
    Ok((anchors, directory))
}

#[cfg(windows)]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
pub(super) fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    use cap_fs_ext::OsMetadataExt as _;

    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
pub(super) fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
