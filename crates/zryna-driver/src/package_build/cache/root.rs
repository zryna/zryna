use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use cap_fs_ext::DirExt as _;
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;

use super::PackageBuildError;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Retained capability for one project-owned `.zryna/cache` directory.
pub struct ArtifactCacheRoot {
    path: PathBuf,
    identity: Arc<Handle>,
}

impl ArtifactCacheRoot {
    /// Creates and retains the exact cache directory below an absolute project root.
    ///
    /// # Errors
    ///
    /// Returns a cache failure when the project or cache directory chain is unsafe or unavailable.
    pub fn prepare_for_project(project_root: &Path) -> Result<Self, PackageBuildError> {
        if !project_root.is_absolute() {
            return Err(PackageBuildError::cache("project root must be absolute"));
        }
        let declared_identity = Handle::from_path(project_root)
            .map_err(|_| PackageBuildError::cache("project root identity cannot be retained"))?;
        crate::javascript::validate_real_directory_chain(project_root)
            .map_err(|_| PackageBuildError::cache("project root is not a safe directory chain"))?;
        let project = Dir::open_ambient_dir(project_root, ambient_authority())
            .map_err(|_| PackageBuildError::cache("project root cannot be retained"))?;
        if directory_identity(&project)? != declared_identity {
            return Err(PackageBuildError::cache("project root changed before capability capture"));
        }
        let state = project_root.join(".zryna");
        create_directory(&project, ".zryna", "project state directory")?;
        let state_directory = project
            .open_dir_nofollow(".zryna")
            .map_err(|_| PackageBuildError::cache("project state directory is unsafe"))?;
        let path = state.join("cache");
        create_directory(&state_directory, "cache", "project cache directory")?;
        let cache_directory = state_directory
            .open_dir_nofollow("cache")
            .map_err(|_| PackageBuildError::cache("project cache directory is unsafe"))?;
        let identity = directory_identity(&cache_directory)?;
        Ok(Self { path, identity: Arc::new(identity) })
    }

    #[must_use]
    /// Returns the retained project cache path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn revalidate(&self) -> Result<(), PackageBuildError> {
        crate::javascript::validate_real_directory_chain(&self.path)
            .map_err(|_| PackageBuildError::cache("cache root is no longer safe"))?;
        let current = Handle::from_path(&self.path)
            .map_err(|_| PackageBuildError::cache("cache root cannot be revalidated"))?;
        if current != *self.identity {
            return Err(PackageBuildError::cache("cache root identity changed"));
        }
        Ok(())
    }

    fn retained_directory(&self) -> Result<Dir, PackageBuildError> {
        self.retained_directory_with_hook(|| {})
    }

    fn retained_directory_with_hook(
        &self,
        before_open: impl FnOnce(),
    ) -> Result<Dir, PackageBuildError> {
        self.revalidate()?;
        before_open();
        let directory = Dir::open_ambient_dir(&self.path, ambient_authority())
            .map_err(|_| PackageBuildError::cache("cache root cannot be retained"))?;
        if directory_identity(&directory)? != *self.identity {
            return Err(PackageBuildError::cache("cache root changed before capability capture"));
        }
        Ok(directory)
    }

    #[cfg(test)]
    pub(crate) fn retained_directory_with_substitution_hook(
        &self,
        before_open: impl FnOnce(),
    ) -> Result<Dir, PackageBuildError> {
        self.retained_directory_with_hook(before_open)
    }

    pub(super) fn retained_build_namespace(&self) -> Result<Dir, PackageBuildError> {
        self.retained_build_namespace_with_hook(|| {})
    }

    fn retained_build_namespace_with_hook(
        &self,
        before_open: impl FnOnce(),
    ) -> Result<Dir, PackageBuildError> {
        let cache_directory = self.retained_directory()?;
        create_directory(&cache_directory, "build-plan-v0", "cache namespace")?;
        before_open();
        cache_directory
            .open_dir_nofollow("build-plan-v0")
            .map_err(|_| PackageBuildError::cache("cache namespace is unsafe"))
    }

    #[cfg(test)]
    pub(crate) fn retained_build_namespace_with_substitution_hook(
        &self,
        before_open: impl FnOnce(),
    ) -> Result<Dir, PackageBuildError> {
        self.retained_build_namespace_with_hook(before_open)
    }
}

fn create_directory(parent: &Dir, name: &str, label: &str) -> Result<(), PackageBuildError> {
    match parent.create_dir(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(_) => {
            let detail = format!("{label} cannot be prepared");
            Err(PackageBuildError::cache(&detail))
        }
    }
}

fn directory_identity(directory: &Dir) -> Result<Handle, PackageBuildError> {
    directory
        .try_clone()
        .map(Dir::into_std_file)
        .and_then(Handle::from_file)
        .map_err(|_| PackageBuildError::cache("directory identity cannot be retained"))
}
