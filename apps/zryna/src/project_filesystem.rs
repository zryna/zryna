use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read as _, Seek as _, SeekFrom, Write as _},
    path::{Component, Path, PathBuf},
};

use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;

use crate::project::ProjectError;

pub(super) fn publish(
    destination: &Path,
    name: &str,
    manifest: &[u8],
    lock: &[u8],
    source: &[u8],
) -> Result<(), ProjectError> {
    publish_with_checkpoint(destination, name, manifest, lock, source, || {})
}

fn publish_with_checkpoint(
    destination: &Path,
    name: &str,
    manifest: &[u8],
    lock: &[u8],
    source: &[u8],
    checkpoint: impl FnOnce(),
) -> Result<(), ProjectError> {
    let parent_path = destination
        .parent()
        .ok_or_else(|| ProjectError::request("project destination must have a parent"))?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| ProjectError::request("project destination must have a name"))?;
    let parent = CapturedParent::capture(parent_path)?;
    parent.require_absent(destination_name, "project destination already exists")?;
    let stage_name = OsString::from(format!(".zryna-new-{name}.pending"));
    parent.require_absent(&stage_name, "project staging destination already exists")?;
    let stage = parent.create_stage(&stage_name)?;
    let source_directory = match stage.create_source_directory() {
        Ok(source) => source,
        Err(error) => {
            if stage.cleanup(&parent, &stage_name, None).is_err() {
                return Err(ProjectError::cleanup(
                    "project source creation failed and its retained stage could not be removed",
                ));
            }
            return Err(error);
        }
    };
    let result = (|| {
        let manifest_file = write_new(&stage.directory, "zryna.package.json", manifest)?;
        let lock_file = write_new(&stage.directory, "zryna.lock.json", lock)?;
        let source_file = write_new(&source_directory.directory, "main.zry", source)?;
        sync_directory(&source_directory.directory)?;
        sync_directory(&stage.directory)?;
        checkpoint();
        parent.revalidate()?;
        stage.revalidate(&parent, &stage_name)?;
        source_directory.revalidate(&stage)?;
        validate_inventory(&stage.directory, &["src", "zryna.lock.json", "zryna.package.json"])?;
        validate_inventory(&source_directory.directory, &["main.zry"])?;
        manifest_file.revalidate(manifest)?;
        lock_file.revalidate(lock)?;
        source_file.revalidate(source)?;
        rename_noreplace(&parent, &stage_name, destination_name)?;
        Ok(())
    })();
    if result.is_err() && stage.cleanup(&parent, &stage_name, Some(&source_directory)).is_err() {
        return Err(ProjectError::cleanup(
            "project creation failed and its retained stage could not be removed",
        ));
    }
    result
}

struct CapturedParent {
    path: PathBuf,
    directory: Dir,
    identity: Handle,
    _anchors: Vec<Dir>,
}

impl CapturedParent {
    fn capture(path: &Path) -> Result<Self, ProjectError> {
        if !path.is_absolute() {
            return Err(ProjectError::request("project parent must be absolute"));
        }
        let (anchors, directory) = capture_absolute_root(path)
            .map_err(|_| ProjectError::request("project parent cannot be retained safely"))?;
        let identity = directory_identity(&directory)
            .map_err(|_| ProjectError::request("project parent identity is unavailable"))?;
        Ok(Self { path: path.to_path_buf(), directory, identity, _anchors: anchors })
    }

    fn revalidate(&self) -> Result<(), ProjectError> {
        let (_, reopened) = capture_absolute_root(&self.path)
            .map_err(|_| ProjectError::publication("project parent cannot be revalidated"))?;
        let current = directory_identity(&self.directory)
            .map_err(|_| ProjectError::publication("project parent identity changed"))?;
        let reopened = directory_identity(&reopened)
            .map_err(|_| ProjectError::publication("project parent identity changed"))?;
        if current == self.identity && reopened == self.identity {
            Ok(())
        } else {
            Err(ProjectError::publication("project parent identity changed before commit"))
        }
    }

    fn require_absent(&self, name: &OsStr, message: &'static str) -> Result<(), ProjectError> {
        match self.directory.symlink_metadata(name) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(ProjectError::collision(message)),
            Err(_) => Err(ProjectError::request("project destination cannot be inspected")),
        }
    }

    fn create_stage(&self, name: &OsStr) -> Result<RetainedStage, ProjectError> {
        self.directory
            .create_dir(name)
            .map_err(|_| ProjectError::publication("project stage cannot be created"))?;
        let directory = self.directory.open_dir_nofollow(name).map_err(|_| {
            ProjectError::cleanup("created project stage could not be retained safely")
        })?;
        let identity = directory_identity(&directory)
            .map_err(|_| ProjectError::cleanup("created project stage identity is unavailable"))?;
        Ok(RetainedStage { directory, identity })
    }
}

struct RetainedStage {
    directory: Dir,
    identity: Handle,
}

impl RetainedStage {
    fn revalidate(&self, parent: &CapturedParent, name: &OsStr) -> Result<(), ProjectError> {
        let reopened = parent.directory.open_dir_nofollow(name).map_err(|_| {
            ProjectError::publication("project stage changed before create-only commit")
        })?;
        let current = directory_identity(&self.directory)
            .map_err(|_| ProjectError::publication("project stage identity changed"))?;
        let reopened = directory_identity(&reopened)
            .map_err(|_| ProjectError::publication("project stage identity changed"))?;
        if current == self.identity && reopened == self.identity {
            Ok(())
        } else {
            Err(ProjectError::publication("project stage identity changed before commit"))
        }
    }

    fn create_source_directory(&self) -> Result<RetainedSourceDirectory, ProjectError> {
        self.directory
            .create_dir("src")
            .map_err(|_| ProjectError::publication("project source directory cannot be created"))?;
        let directory = self.directory.open_dir_nofollow("src").map_err(|_| {
            ProjectError::cleanup("created project source directory cannot be retained")
        })?;
        let identity = directory_identity(&directory).map_err(|_| {
            ProjectError::cleanup("created project source directory identity is unavailable")
        })?;
        Ok(RetainedSourceDirectory { directory, identity })
    }

    fn cleanup(
        &self,
        parent: &CapturedParent,
        name: &OsStr,
        source: Option<&RetainedSourceDirectory>,
    ) -> io::Result<()> {
        let reopened = parent.directory.open_dir_nofollow(name)?;
        if directory_identity(&reopened)? != self.identity
            || directory_identity(&self.directory)? != self.identity
        {
            return Err(io::Error::other("project stage identity changed"));
        }
        remove_file_if_present(&self.directory, "zryna.package.json")?;
        remove_file_if_present(&self.directory, "zryna.lock.json")?;
        match source {
            Some(source) => {
                source.revalidate_io(self)?;
                remove_file_if_present(&source.directory, "main.zry")?;
                self.directory.remove_dir("src")?;
            }
            None => match self.directory.symlink_metadata("src") {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(_) => return Err(io::Error::other("unretained project source directory")),
                Err(error) => return Err(error),
            },
        }
        parent.directory.remove_dir(name)
    }
}

struct RetainedSourceDirectory {
    directory: Dir,
    identity: Handle,
}

impl RetainedSourceDirectory {
    fn revalidate(&self, stage: &RetainedStage) -> Result<(), ProjectError> {
        self.revalidate_io(stage)
            .map_err(|_| ProjectError::publication("project source directory identity changed"))
    }

    fn revalidate_io(&self, stage: &RetainedStage) -> io::Result<()> {
        let current = stage.directory.open_dir_nofollow("src")?;
        if directory_identity(&current)? == self.identity
            && directory_identity(&self.directory)? == self.identity
        {
            Ok(())
        } else {
            Err(io::Error::other("project source directory identity changed"))
        }
    }
}

struct RetainedFile {
    directory: Dir,
    name: String,
    identity: Handle,
}

impl RetainedFile {
    fn revalidate(&self, expected: &[u8]) -> Result<(), ProjectError> {
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let current = self
            .directory
            .open_with(&self.name, &options)
            .map(cap_std::fs::File::into_std)
            .and_then(Handle::from_file)
            .map_err(|_| ProjectError::publication("project file identity changed"))?;
        if current != self.identity {
            return Err(ProjectError::publication("project file identity changed"));
        }
        let mut file = current
            .as_file()
            .try_clone()
            .map_err(|_| ProjectError::publication("project file cannot be revalidated"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|_| ProjectError::publication("project file cannot be revalidated"))?;
        let mut bytes = Vec::with_capacity(expected.len());
        file.take(u64::try_from(expected.len()).unwrap_or(u64::MAX).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ProjectError::publication("project file cannot be revalidated"))?;
        if bytes == expected {
            Ok(())
        } else {
            Err(ProjectError::publication("project file bytes changed before commit"))
        }
    }
}

fn write_new(directory: &Dir, name: &str, bytes: &[u8]) -> Result<RetainedFile, ProjectError> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(name, &options)
        .map_err(|_| ProjectError::publication("project file cannot be created"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| ProjectError::publication("project file cannot be synchronized"))?;
    let metadata = directory
        .symlink_metadata(name)
        .map_err(|_| ProjectError::publication("project file cannot be revalidated"))?;
    if !metadata.is_file() || cap_metadata_is_link_or_reparse(&metadata) {
        return Err(ProjectError::publication("project file identity changed during creation"));
    }
    let identity = Handle::from_file(file.into_std())
        .map_err(|_| ProjectError::publication("created project file identity is unavailable"))?;
    let directory = directory
        .try_clone()
        .map_err(|_| ProjectError::publication("project directory cannot be retained"))?;
    Ok(RetainedFile { directory, name: name.to_owned(), identity })
}

fn validate_inventory(directory: &Dir, expected: &[&str]) -> Result<(), ProjectError> {
    let mut names = directory
        .entries()
        .map_err(|_| ProjectError::publication("project stage cannot be enumerated"))?
        .map(|entry| {
            entry
                .map_err(|_| ProjectError::publication("project stage entry cannot be read"))
                .and_then(|entry| {
                    entry.file_name().into_string().map_err(|_| {
                        ProjectError::publication("project stage entry is not portable UTF-8")
                    })
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    let expected = expected.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
    if names == expected {
        Ok(())
    } else {
        Err(ProjectError::publication("project stage contains an unexpected entry"))
    }
}

fn remove_file_if_present(directory: &Dir, name: &str) -> io::Result<()> {
    match directory.remove_file(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn directory_identity(directory: &Dir) -> io::Result<Handle> {
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
fn capture_absolute_root(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
    use std::path::Prefix;

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
fn capture_absolute_root(path: &Path) -> io::Result<(Vec<Dir>, Dir)> {
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

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn rename_noreplace(
    parent: &CapturedParent,
    source: &OsStr,
    destination: &OsStr,
) -> Result<(), ProjectError> {
    use nix::fcntl::{RenameFlags, renameat2};

    let directory = parent
        .directory
        .try_clone()
        .map(Dir::into_std_file)
        .map_err(|_| ProjectError::publication("project parent cannot be retained"))?;
    renameat2(&directory, source, &directory, destination, RenameFlags::RENAME_NOREPLACE).map_err(
        |error| {
            if matches!(error, nix::errno::Errno::EEXIST | nix::errno::Errno::ENOTEMPTY) {
                ProjectError::collision("project destination appeared before commit")
            } else {
                ProjectError::publication("create-only project commit failed")
            }
        },
    )
}

#[cfg(windows)]
fn rename_noreplace(
    parent: &CapturedParent,
    source: &OsStr,
    destination: &OsStr,
) -> Result<(), ProjectError> {
    parent.directory.rename(source, &parent.directory, destination).map_err(|error| {
        if matches!(error.kind(), io::ErrorKind::AlreadyExists | io::ErrorKind::DirectoryNotEmpty) {
            ProjectError::collision("project destination appeared before commit")
        } else {
            ProjectError::publication("create-only project commit failed")
        }
    })
}

#[cfg(not(any(windows, all(target_os = "linux", target_env = "gnu"))))]
fn rename_noreplace(
    _parent: &CapturedParent,
    _source: &OsStr,
    _destination: &OsStr,
) -> Result<(), ProjectError> {
    Err(ProjectError::publication("create-only directory commit is unsupported on this platform"))
}

#[cfg(unix)]
fn sync_directory(directory: &Dir) -> Result<(), ProjectError> {
    directory
        .open(".")
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ProjectError::publication("project directory cannot be synchronized"))
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Dir) -> Result<(), ProjectError> {
    Ok(())
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
fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    use cap_fs_ext::MetadataExt as _;

    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(all(test, unix))]
#[path = "project_filesystem_tests.rs"]
mod tests;
