use std::{
    ffi::{OsStr, OsString},
    io::{self, Read as _, Seek as _, SeekFrom, Write as _},
    path::{Path, PathBuf},
};

#[cfg(not(windows))]
use cap_fs_ext::DirExt as _;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::Dir;
use same_file::Handle;
#[cfg(windows)]
use zryna_windows_filesystem::{OwnedDirectory, create_directory};

use crate::project::ProjectError;
use project_filesystem_capture::{
    cap_metadata_is_link_or_reparse, capture_absolute_root, directory_identity,
};
#[cfg(windows)]
use project_filesystem_windows::{ProjectContents, publish_windows_stage};

#[path = "project_filesystem_capture.rs"]
mod project_filesystem_capture;

#[cfg(windows)]
#[path = "project_filesystem_windows.rs"]
mod project_filesystem_windows;

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
            #[cfg(windows)]
            let cleanup = stage.cleanup(None);
            #[cfg(not(windows))]
            let cleanup = stage.cleanup(&parent, &stage_name, None);
            if cleanup.is_err() {
                return Err(ProjectError::cleanup(
                    "project source creation failed and its retained stage could not be removed",
                ));
            }
            return Err(error);
        }
    };
    #[cfg(windows)]
    {
        publish_windows_stage(
            stage,
            source_directory,
            &parent,
            destination_name,
            ProjectContents { manifest, lock, source },
            checkpoint,
        )
    }
    #[cfg(not(windows))]
    {
        let mut stage = stage;
        let result = (|| {
            let manifest_file = write_new(stage.directory(), "zryna.package.json", manifest)?;
            let lock_file = write_new(stage.directory(), "zryna.lock.json", lock)?;
            let source_file = write_new(source_directory.directory(), "main.zry", source)?;
            #[cfg(unix)]
            {
                sync_directory(source_directory.directory())?;
                sync_directory(stage.directory())?;
            }
            checkpoint();
            parent.revalidate()?;
            stage.revalidate(&parent, &stage_name)?;
            source_directory.revalidate(&stage)?;
            validate_inventory(
                stage.directory(),
                &["src", "zryna.lock.json", "zryna.package.json"],
            )?;
            validate_inventory(source_directory.directory(), &["main.zry"])?;
            manifest_file.revalidate(manifest)?;
            lock_file.revalidate(lock)?;
            source_file.revalidate(source)?;
            stage.commit(&parent, &stage_name, destination_name)?;
            Ok(())
        })();
        if result.is_err() && stage.cleanup(&parent, &stage_name, Some(source_directory)).is_err() {
            return Err(ProjectError::cleanup(
                "project creation failed and its retained stage could not be removed",
            ));
        }
        result
    }
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

    #[cfg(windows)]
    fn create_stage(&self, name: &OsStr) -> Result<RetainedStage, ProjectError> {
        let owned = create_directory(&self.directory, name)
            .map_err(|_| ProjectError::publication("project stage cannot be created"))?;
        Ok(RetainedStage { owned })
    }

    #[cfg(not(windows))]
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
    #[cfg(windows)]
    owned: OwnedDirectory,
    #[cfg(not(windows))]
    directory: Dir,
    #[cfg(not(windows))]
    identity: Handle,
}

impl RetainedStage {
    #[cfg(windows)]
    fn directory(&self) -> &Dir {
        self.owned.directory()
    }

    #[cfg(not(windows))]
    fn directory(&self) -> &Dir {
        &self.directory
    }

    #[cfg(not(windows))]
    fn revalidate(&self, parent: &CapturedParent, name: &OsStr) -> Result<(), ProjectError> {
        self.revalidate_io(parent, name)
            .map_err(|_| ProjectError::publication("project stage identity changed before commit"))
    }

    #[cfg(not(windows))]
    fn revalidate_io(&self, parent: &CapturedParent, name: &OsStr) -> io::Result<()> {
        let reopened = parent
            .directory
            .open_dir_nofollow(name)
            .map_err(|_| io::Error::other("project stage changed before create-only commit"))?;
        let current = directory_identity(self.directory())?;
        let reopened = directory_identity(&reopened)?;
        if current == self.identity && reopened == self.identity {
            Ok(())
        } else {
            Err(io::Error::other("project stage identity changed"))
        }
    }

    #[cfg(windows)]
    fn create_source_directory(&self) -> Result<RetainedSourceDirectory, ProjectError> {
        let owned = create_directory(self.directory(), OsStr::new("src"))
            .map_err(|_| ProjectError::publication("project source directory cannot be created"))?;
        Ok(RetainedSourceDirectory { owned })
    }

    #[cfg(not(windows))]
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

    #[cfg(windows)]
    fn cleanup(self, source: Option<RetainedSourceDirectory>) -> io::Result<()> {
        remove_file_if_present(self.directory(), "zryna.package.json")?;
        remove_file_if_present(self.directory(), "zryna.lock.json")?;
        match source {
            Some(source) => {
                remove_file_if_present(source.directory(), "main.zry")?;
                source.owned.remove_empty()?;
            }
            None => match self.directory().symlink_metadata("src") {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(_) => return Err(io::Error::other("unretained project source directory")),
                Err(error) => return Err(error),
            },
        }
        self.owned.remove_empty()
    }

    #[cfg(not(windows))]
    fn cleanup(
        self,
        parent: &CapturedParent,
        name: &OsStr,
        source: Option<RetainedSourceDirectory>,
    ) -> io::Result<()> {
        self.revalidate_io(parent, name)?;
        remove_file_if_present(self.directory(), "zryna.package.json")?;
        remove_file_if_present(self.directory(), "zryna.lock.json")?;
        match source {
            Some(source) => {
                source.revalidate_io(&self)?;
                remove_file_if_present(source.directory(), "main.zry")?;
                self.directory.remove_dir("src")?;
            }
            None => match self.directory().symlink_metadata("src") {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(_) => return Err(io::Error::other("unretained project source directory")),
                Err(error) => return Err(error),
            },
        }
        parent.directory.remove_dir(name)
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    fn commit(
        &mut self,
        parent: &CapturedParent,
        source: &OsStr,
        destination: &OsStr,
    ) -> Result<(), ProjectError> {
        self.revalidate(parent, source)?;
        rename_noreplace(parent, source, destination)
    }

    #[cfg(not(any(windows, all(target_os = "linux", target_env = "gnu"))))]
    fn commit(
        &mut self,
        _parent: &CapturedParent,
        _source: &OsStr,
        _destination: &OsStr,
    ) -> Result<(), ProjectError> {
        Err(ProjectError::publication(
            "create-only directory commit is unsupported on this platform",
        ))
    }
}

struct RetainedSourceDirectory {
    #[cfg(windows)]
    owned: OwnedDirectory,
    #[cfg(not(windows))]
    directory: Dir,
    #[cfg(not(windows))]
    identity: Handle,
}

impl RetainedSourceDirectory {
    #[cfg(windows)]
    fn directory(&self) -> &Dir {
        self.owned.directory()
    }

    #[cfg(not(windows))]
    fn directory(&self) -> &Dir {
        &self.directory
    }

    #[cfg(not(windows))]
    fn revalidate(&self, stage: &RetainedStage) -> Result<(), ProjectError> {
        self.revalidate_io(stage)
            .map_err(|_| ProjectError::publication("project source directory identity changed"))
    }

    #[cfg(not(windows))]
    fn revalidate_io(&self, stage: &RetainedStage) -> io::Result<()> {
        let current = stage.directory().open_dir_nofollow("src")?;
        if directory_identity(&current)? == self.identity
            && directory_identity(self.directory())? == self.identity
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

#[cfg(unix)]
fn sync_directory(directory: &Dir) -> Result<(), ProjectError> {
    directory
        .open(".")
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ProjectError::publication("project directory cannot be synchronized"))
}

#[cfg(test)]
#[path = "project_filesystem_tests.rs"]
mod tests;
