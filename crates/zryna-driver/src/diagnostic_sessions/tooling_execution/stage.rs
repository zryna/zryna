use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, File};
use same_file::Handle;
use zryna_diagnostics::Diagnostic;

use super::{
    capture::{CapturedToolingClosure, V4Layout},
    execution_error,
};

mod creation;
#[cfg(test)]
mod creation_tests;
mod filesystem;
mod inventory;
mod v4;
#[cfg(test)]
mod v4_tests;

use creation::{
    create_directory, create_root, open_owned_regular, original_identity, seal_directory_states,
    stage_file,
};
#[cfg(target_os = "linux")]
use filesystem::capability_root;
use filesystem::{
    configure_create, configure_read, directory_identity, hash_handle, metadata_is_link_or_reparse,
    open_regular, same_file_state, validate_root_path,
};
use inventory::{cleanup_keys, validate_inventory};

const ROOT: &str = "";
const MODULES: &str = "node_modules";
const SCOPE: &str = "node_modules/@typescript";
const WRAPPER: &str = "node_modules/@typescript/typescript6";
const WRAPPER_LIB: &str = "node_modules/@typescript/typescript6/lib";
const OLD: &str = "node_modules/@typescript/old";
const OLD_LIB: &str = "node_modules/@typescript/old/lib";

#[derive(Debug)]
struct RetainedDirectory {
    parent: Option<&'static str>,
    name: String,
    original: Option<Dir>,
    identity: Option<Handle>,
    state: Option<fs::Metadata>,
}

impl RetainedDirectory {
    fn pending(parent: Option<&'static str>, name: String) -> Self {
        Self { parent, name, original: None, identity: None, state: None }
    }

    fn directory(&self) -> Result<&Dir, Diagnostic> {
        self.original.as_ref().ok_or_else(stage_changed)
    }
}

#[derive(Debug)]
struct RetainedFile {
    parent: &'static str,
    name: &'static str,
    original: File,
    identity: Option<Handle>,
    state: Option<fs::Metadata>,
    sha256: [u8; 32],
}

/// Fixed authenticated worker/dependency stage. Unix owner permissions and Windows inherited private ACLs are trusted.
#[derive(Debug)]
pub(super) struct ToolingStage {
    v4_layout: V4Layout,
    path: PathBuf,
    worker: PathBuf,
    working_directory: PathBuf,
    root_parent: Dir,
    directories: BTreeMap<&'static str, RetainedDirectory>,
    files: BTreeMap<&'static str, RetainedFile>,
    cleanup_attempted: bool,
}

impl ToolingStage {
    pub(super) fn create(captured: &CapturedToolingClosure) -> Result<Self, Diagnostic> {
        captured.validate_size()?;
        let layout = v4::layout(&captured.v4)?;
        let mut stage = create_root()?;
        stage.v4_layout = layout;
        let prepared = (|| {
            create_directory(&mut stage, ROOT, "node_modules", MODULES)?;
            create_directory(&mut stage, MODULES, "@typescript", SCOPE)?;
            create_directory(&mut stage, SCOPE, "typescript6", WRAPPER)?;
            create_directory(&mut stage, WRAPPER, "lib", WRAPPER_LIB)?;
            create_directory(&mut stage, SCOPE, "old", OLD)?;
            create_directory(&mut stage, OLD, "lib", OLD_LIB)?;
            for (parent, name, file) in [
                (ROOT, "worker.mjs", &captured.worker),
                (ROOT, "worker-v3.mjs", &captured.worker_v3),
                (ROOT, "limits-v3.mjs", &captured.limits_v3),
                (ROOT, "worker-v4.mjs", &captured.worker_v4),
                (ROOT, "limits-v4.mjs", &captured.limits_v4),
                (WRAPPER, "package.json", &captured.wrapper_manifest),
                (WRAPPER_LIB, "typescript.js", &captured.wrapper),
                (OLD, "package.json", &captured.typescript_manifest),
                (OLD_LIB, "typescript.js", &captured.typescript),
            ] {
                stage_file(&mut stage, parent, name, file)?;
            }
            v4::stage(&mut stage, captured)?;
            seal_directory_states(&mut stage.directories)?;
            #[cfg(target_os = "linux")]
            {
                stage.working_directory = capability_root(&stage.directories)?;
                stage.worker = stage.working_directory.join("worker.mjs");
            }
            stage.revalidate()
        })();
        match prepared {
            Ok(()) => Ok(stage),
            Err(primary) => Err(with_cleanup(primary, stage.abort())),
        }
    }

    pub(super) fn worker(&self) -> &Path {
        &self.worker
    }

    pub(super) fn worker_v3(&self) -> PathBuf {
        self.working_directory.join("worker-v3.mjs")
    }

    pub(super) fn worker_v4(&self) -> PathBuf {
        self.working_directory.join("worker-v4.mjs")
    }

    pub(super) fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    #[cfg(test)]
    pub(super) fn physical_path(&self) -> &Path {
        &self.path
    }

    fn bound_directory(&self, key: &'static str) -> Result<Dir, Diagnostic> {
        let retained = self.directories.get(key).ok_or_else(stage_changed)?;
        let current = match retained.parent {
            Some(parent) => self
                .bound_directory(parent)?
                .open_dir_nofollow(&retained.name)
                .map_err(|_| stage_changed())?,
            None => {
                self.root_parent.open_dir_nofollow(&retained.name).map_err(|_| stage_changed())?
            }
        };
        if directory_identity(&current)? != directory_identity(retained.directory()?)? {
            return Err(stage_changed());
        }
        Ok(current)
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        let root = self.directories.get(ROOT).ok_or_else(stage_changed)?;
        if directory_identity(&validate_root_path(&self.path)?)?
            != directory_identity(root.directory()?)?
        {
            return Err(stage_changed());
        }
        for (key, directory) in &self.directories {
            let current = self.bound_directory(key)?;
            let identity = directory_identity(&current)?;
            let state = identity.as_file().metadata().map_err(|_| stage_changed())?;
            if Some(&identity) != directory.identity.as_ref()
                || !same_file_state(&state, directory.state.as_ref().ok_or_else(stage_changed)?)
                || !state.is_dir()
                || metadata_is_link_or_reparse(&state)
            {
                return Err(stage_changed());
            }
            validate_inventory(key, directory.directory()?, self.v4_layout)?;
        }
        for file in self.files.values() {
            let parent = self.bound_directory(file.parent)?;
            let mut current = open_regular(&parent, file.name)?;
            let state = current.as_file().metadata().map_err(|_| stage_changed())?;
            if Some(&current) != file.identity.as_ref()
                || !same_file_state(&state, file.state.as_ref().ok_or_else(stage_changed)?)
                || hash_handle(&mut current)? != file.sha256
            {
                return Err(stage_changed());
            }
        }
        Ok(())
    }

    /// Consuming explicit cleanup. A failed cleanup is retained as an error, never retried by Drop.
    pub(super) fn abort(mut self) -> Result<(), Diagnostic> {
        self.cleanup_attempted = true;
        self.cleanup()
    }

    fn cleanup_file(&mut self, key: &'static str) -> Result<(), Diagnostic> {
        let file = self.files.get(key).ok_or_else(stage_changed)?;
        let parent = self.bound_directory(file.parent)?;
        let original = original_identity(&file.original)?;
        let current = original_identity(&open_owned_regular(&parent, file.name)?)?;
        if current != original {
            return Err(stage_changed());
        }
        let name = file.name;
        drop(current);
        drop(original);
        drop(self.files.remove(key));
        parent.remove_file(name).map_err(|_| stage_changed())
    }

    fn cleanup_directory(&mut self, key: &'static str) -> Result<(), Diagnostic> {
        let directory = self.directories.get(key).ok_or_else(stage_changed)?;
        let parent = match directory.parent {
            Some(parent) => self.bound_directory(parent)?,
            None => self.root_parent.try_clone().map_err(|_| stage_changed())?,
        };
        let current = self.bound_directory(key)?;
        if current
            .entries()
            .map_err(|_| stage_changed())?
            .next()
            .transpose()
            .map_err(|_| stage_changed())?
            .is_some()
        {
            return Err(stage_changed());
        }
        let name = directory.name.clone();
        drop(current);
        drop(self.directories.remove(key));
        parent.remove_dir(name).map_err(|_| stage_changed())
    }

    fn cleanup(&mut self) -> Result<(), Diagnostic> {
        let mut failures = Vec::new();
        for key in cleanup_keys(self.v4_layout) {
            if self.files.contains_key(key)
                && let Err(error) = self.cleanup_file(key)
            {
                failures.push(format!("file {key}: {}", error.message()));
            }
        }
        for key in [
            v4::SYNTAX,
            v4::BOUNDARY,
            v4::V4,
            OLD_LIB,
            OLD,
            WRAPPER_LIB,
            WRAPPER,
            SCOPE,
            MODULES,
            ROOT,
        ] {
            let name = self.directories.get(key).map(|directory| directory.name.clone());
            if self.directories.contains_key(key)
                && let Err(error) = self.cleanup_directory(key)
            {
                failures.push(format!("directory {key:?} ({name:?}): {}", error.message()));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(execution_error(format!(
                "tooling stage cleanup incomplete: {}",
                failures.join("; ")
            )))
        }
    }
}

impl Drop for ToolingStage {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            self.cleanup_attempted = true;
            let _ = self.cleanup();
        }
    }
}

fn with_cleanup(primary: Diagnostic, cleanup: Result<(), Diagnostic>) -> Diagnostic {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => Diagnostic::error(
            primary.code(),
            primary.path().map(str::to_owned),
            format!("{}; cleanup [{}]: {}", primary.message(), cleanup.code(), cleanup.message()),
            primary.guidance(),
        ),
    }
}

fn stage_changed() -> Diagnostic {
    execution_error("tooling private-stage identity, inventory, or executable bytes changed")
}
