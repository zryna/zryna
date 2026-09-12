use std::{ffi::OsStr, io};

use crate::project::ProjectError;

use super::{
    CapturedParent, RetainedFile, RetainedSourceDirectory, RetainedStage, validate_inventory,
    write_new,
};

#[derive(Clone, Copy)]
pub(super) struct ProjectContents<'a> {
    pub(super) manifest: &'a [u8],
    pub(super) lock: &'a [u8],
    pub(super) source: &'a [u8],
}

pub(super) fn publish_windows_stage(
    stage: RetainedStage,
    source_directory: RetainedSourceDirectory,
    parent: &CapturedParent,
    destination_name: &OsStr,
    contents: ProjectContents<'_>,
    checkpoint: impl FnOnce(),
) -> Result<(), ProjectError> {
    let manifest_file = match write_new(stage.directory(), "zryna.package.json", contents.manifest)
    {
        Ok(file) => file,
        Err(error) => {
            return cleanup_incomplete(stage, source_directory, error);
        }
    };
    let lock_file = match write_new(stage.directory(), "zryna.lock.json", contents.lock) {
        Ok(file) => file,
        Err(error) => {
            drop(manifest_file);
            return cleanup_incomplete(stage, source_directory, error);
        }
    };
    let source_file = match write_new(source_directory.directory(), "main.zry", contents.source) {
        Ok(file) => file,
        Err(error) => {
            drop(lock_file);
            drop(manifest_file);
            return cleanup_incomplete(stage, source_directory, error);
        }
    };
    let open = OpenStage { stage, source_directory, manifest_file, lock_file, source_file };
    checkpoint();
    let (open, validation) = open.validate(parent, contents);
    match validation {
        Ok(()) => open.seal().commit(parent, destination_name),
        Err(error) => match open.cleanup() {
            Ok(()) => Err(error),
            Err(_) => Err(cleanup_error()),
        },
    }
}

fn cleanup_incomplete(
    stage: RetainedStage,
    source_directory: RetainedSourceDirectory,
    error: ProjectError,
) -> Result<(), ProjectError> {
    match stage.cleanup(Some(source_directory)) {
        Ok(()) => Err(error),
        Err(_) => Err(cleanup_error()),
    }
}

fn cleanup_error() -> ProjectError {
    ProjectError::cleanup("project creation failed and its retained stage could not be removed")
}

impl RetainedStage {
    fn commit(&mut self, parent: &CapturedParent, destination: &OsStr) -> Result<(), ProjectError> {
        self.owned.rename_noreplace(&parent.directory, destination).map_err(|error| {
            if matches!(
                error.kind(),
                io::ErrorKind::AlreadyExists | io::ErrorKind::DirectoryNotEmpty
            ) {
                ProjectError::collision("project destination appeared before commit")
            } else {
                ProjectError::publication("create-only project commit failed")
            }
        })
    }
}

struct OpenStage {
    stage: RetainedStage,
    source_directory: RetainedSourceDirectory,
    manifest_file: RetainedFile,
    lock_file: RetainedFile,
    source_file: RetainedFile,
}

impl OpenStage {
    fn validate(
        self,
        parent: &CapturedParent,
        contents: ProjectContents<'_>,
    ) -> (Self, Result<(), ProjectError>) {
        let validation: Result<(), ProjectError> = (|| {
            parent.revalidate()?;
            validate_inventory(
                self.stage.directory(),
                &["src", "zryna.lock.json", "zryna.package.json"],
            )?;
            validate_inventory(self.source_directory.directory(), &["main.zry"])?;
            self.manifest_file.revalidate(contents.manifest)?;
            self.lock_file.revalidate(contents.lock)?;
            self.source_file.revalidate(contents.source)?;
            Ok(())
        })();
        (self, validation)
    }

    fn seal(self) -> SealedStage {
        let Self { stage, source_directory, manifest_file, lock_file, source_file } = self;
        drop(source_file);
        drop(lock_file);
        drop(manifest_file);
        drop(source_directory);
        SealedStage { stage }
    }

    fn cleanup(self) -> io::Result<()> {
        let Self { stage, source_directory, manifest_file, lock_file, source_file } = self;
        drop(source_file);
        drop(lock_file);
        drop(manifest_file);
        stage.cleanup(Some(source_directory))
    }
}

struct SealedStage {
    stage: RetainedStage,
}

impl SealedStage {
    fn commit(mut self, parent: &CapturedParent, destination: &OsStr) -> Result<(), ProjectError> {
        let cause = match self.stage.commit(parent, destination) {
            Ok(()) => return Ok(()),
            Err(cause) => cause,
        };
        let message =
            format!("{}; sealed project stage was preserved and is untrusted", cause.message);
        drop(self);
        Err(ProjectError::cleanup(message))
    }
}
