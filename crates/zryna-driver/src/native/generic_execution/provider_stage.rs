//! Dedicated two-file private provider directory; no system cache or hwcaps lookup.

mod inputs;
use super::{dependencies::Dependencies, staging_error as rejected};
use crate::native::{
    NativeStage, NativeStageIdentity, ToolFileIdentity, native_stage_identity,
    stage_cleanup_warning,
};
use std::{fs, os::unix::fs::DirBuilderExt as _, path::PathBuf};
use zryna_diagnostics::Diagnostic;

pub(super) struct ProviderStage<'s> {
    parent: &'s NativeStage,
    directory: PathBuf,
    handle: cap_std::fs::Dir,
    identity: NativeStageIdentity,
    files: Vec<(&'static str, ToolFileIdentity)>,
}

impl<'s> ProviderStage<'s> {
    pub(super) fn create(
        parent: &'s NativeStage,
        dependencies: &Dependencies,
    ) -> Result<Self, Vec<Diagnostic>> {
        parent.revalidate().map_err(|error| vec![error])?;
        let directory = parent.capability_directory_path().join("providers");
        fs::DirBuilder::new().mode(0o700).create(&directory).map_err(|_| vec![rejected()])?;
        let result = (|| {
            let file = fs::File::open(&directory).map_err(|_| rejected())?;
            let identity = native_stage_identity(&file.metadata().map_err(|_| rejected())?)?;
            let mut files = Vec::new();
            files.try_reserve_exact(2).map_err(|_| rejected())?;
            Ok(Self {
                parent,
                directory: directory.clone(),
                handle: cap_std::fs::Dir::from_std_file(file),
                identity,
                files,
            })
        })();
        let mut stage = match result {
            Ok(stage) => stage,
            Err(error) => {
                let mut errors = vec![error];
                if parent.directory_handle.remove_dir("providers").is_err() {
                    errors.push(stage_cleanup_warning());
                }
                return Err(errors);
            }
        };
        let result = (|| {
            stage.directory_identity()?;
            dependencies.revalidate()?;
            for (name, bytes) in dependencies.materials() {
                let identity = stage.write(name, bytes)?;
                stage.files.push((name, identity));
            }
            stage.revalidate(dependencies)
        })();
        if let Err(error) = result {
            let mut errors = vec![error];
            errors.extend(stage.cleanup());
            return Err(errors);
        }
        Ok(stage)
    }

    pub(super) fn directory(&self) -> PathBuf {
        use std::os::fd::AsRawFd as _;
        PathBuf::from(format!("/proc/{}/fd/{}", std::process::id(), self.handle.as_raw_fd()))
    }

    pub(super) fn loader(&self) -> PathBuf {
        self.directory().join(super::dependencies::LOADER)
    }

    pub(super) fn directory_identity(&self) -> Result<(), Diagnostic> {
        use cap_std::fs::MetadataExt as _;
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        self.parent.revalidate()?;
        let held = self.handle.dir_metadata().map_err(|_| rejected())?;
        let current = fs::symlink_metadata(&self.directory).map_err(|_| rejected())?;
        if !held.is_dir()
            || !current.is_dir()
            || current.file_type().is_symlink()
            || held.dev() != self.identity.device
            || held.ino() != self.identity.inode
            || current.dev() != self.identity.device
            || current.ino() != self.identity.inode
            || current.permissions().mode() & 0o777 != 0o700
        {
            return Err(rejected());
        }
        Ok(())
    }

    pub(super) fn cleanup(&self) -> Vec<Diagnostic> {
        if self.directory_identity().is_err() {
            return vec![stage_cleanup_warning()];
        }
        let mut failed = false;
        for name in [super::dependencies::LOADER, super::dependencies::LIBC] {
            match self.handle.remove_file(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => failed = true,
            }
        }
        match self.handle.entries() {
            Ok(mut entries) => {
                if entries.next().is_some()
                    || self.parent.directory_handle.remove_dir("providers").is_err()
                {
                    failed = true;
                }
            }
            Err(_) => failed = true,
        }
        if failed { vec![stage_cleanup_warning()] } else { Vec::new() }
    }
}
