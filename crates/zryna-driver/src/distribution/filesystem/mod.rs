//! Retained no-follow installation topology; binary hashes are streamed, provider bytes retained.

mod file;
mod platform;
#[cfg(test)]
mod tests;

pub(super) use platform::absolute as capture_absolute_directory;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use cap_std::fs::Dir;
use same_file::Handle;
use zryna_diagnostics::Diagnostic;

use super::{
    admission_error,
    manifest::{FileRecord, portable},
};
use file::RetainedFile;

struct Directory {
    dir: Dir,
    identity: Handle,
    metadata: fs::Metadata,
}

pub(super) struct InstallationTree {
    path: PathBuf,
    _anchors: Vec<Dir>,
    directories: BTreeMap<String, Directory>,
    files: BTreeMap<String, RetainedFile>,
}

impl InstallationTree {
    pub(super) fn capture(path: &Path) -> Result<Self, Diagnostic> {
        let (anchors, root) = platform::absolute(path).map_err(|_| changed())?;
        let root = retained_directory(root)?;
        Ok(Self {
            path: path.to_owned(),
            _anchors: anchors,
            directories: BTreeMap::from([(String::new(), root)]),
            files: BTreeMap::new(),
        })
    }

    pub(super) fn capture_file(
        &mut self,
        path: &str,
        limit: u64,
        retain: bool,
    ) -> Result<(), Diagnostic> {
        if !portable(path) || self.files.contains_key(path) || self.files.len() >= 512 {
            return Err(changed());
        }
        let parts = path.split('/').collect::<Vec<_>>();
        let mut parent = String::new();
        for part in &parts[..parts.len() - 1] {
            let key =
                if parent.is_empty() { (*part).to_owned() } else { format!("{parent}/{part}") };
            if !self.directories.contains_key(&key) {
                let parent_directory = self.directories.get(&parent).ok_or_else(changed)?;
                let child = platform::child(&parent_directory.dir, std::ffi::OsStr::new(part))
                    .map_err(|_| changed())?;
                self.directories.insert(key.clone(), retained_directory(child)?);
            }
            parent = key;
        }
        let parent = self.directories.get(&parent).ok_or_else(changed)?;
        let name = parts.last().ok_or_else(changed)?;
        let file = RetainedFile::open(&parent.dir, name, limit, retain)?;
        let total: u64 = self.files.values().map(|file| file.metadata.len()).sum();
        if total + file.metadata.len() > 512 * 1024 * 1024
            || self.files.values().any(|other| other.identity == file.identity)
        {
            return Err(changed());
        }
        self.files.insert(path.to_owned(), file);
        Ok(())
    }

    pub(super) fn bytes(&self, path: &str) -> Result<&[u8], Diagnostic> {
        self.files.get(path).and_then(|file| file.bytes.as_deref()).ok_or_else(changed)
    }

    pub(super) fn matches(&self, record: &FileRecord) -> Result<(), Diagnostic> {
        let file = self.files.get(&record.path).ok_or_else(changed)?;
        if file.sha256 == record.sha256
            && file.metadata.len() == record.size
            && file.mode_matches(record.mode)
        {
            Ok(())
        } else {
            Err(changed())
        }
    }

    pub(super) fn digest(&self, path: &str) -> Result<&str, Diagnostic> {
        self.files.get(path).map(|file| file.sha256.as_str()).ok_or_else(changed)
    }

    pub(super) fn size(&self, path: &str) -> Result<u64, Diagnostic> {
        self.files.get(path).map(|file| file.metadata.len()).ok_or_else(changed)
    }

    pub(super) fn mode_matches(&self, path: &str, mode: u32) -> Result<(), Diagnostic> {
        if self.files.get(path).ok_or_else(changed)?.mode_matches(mode) {
            Ok(())
        } else {
            Err(changed())
        }
    }

    pub(super) fn identity(&self, path: &str) -> Result<&Handle, Diagnostic> {
        self.files.get(path).map(|file| &file.identity).ok_or_else(changed)
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        let (anchors, current_root) = platform::absolute(&self.path).map_err(|_| changed())?;
        if anchors.len() != self._anchors.len() {
            return Err(changed());
        }
        for (current, retained) in anchors.iter().zip(&self._anchors) {
            if platform::directory_identity(current).map_err(|_| changed())?
                != platform::directory_identity(retained).map_err(|_| changed())?
            {
                return Err(changed());
            }
        }
        let original = self.directories.get("").ok_or_else(changed)?;
        if platform::directory_identity(&current_root).map_err(|_| changed())? != original.identity
        {
            return Err(changed());
        }
        for (key, directory) in &self.directories {
            let current = if key.is_empty() {
                current_root.try_clone().map_err(|_| changed())?
            } else {
                let (parent, name) = parent_name(key);
                platform::child(
                    &self.directories.get(parent).ok_or_else(changed)?.dir,
                    std::ffi::OsStr::new(name),
                )
                .map_err(|_| changed())?
            };
            let identity = platform::directory_identity(&current).map_err(|_| changed())?;
            let metadata = identity.as_file().metadata().map_err(|_| changed())?;
            if identity != directory.identity
                || !platform::same_state(&metadata, &directory.metadata)
            {
                return Err(changed());
            }
            self.validate_inventory(key, &current)?;
        }
        for (path, file) in &self.files {
            let (parent, name) = parent_name(path);
            let directory = &self.directories.get(parent).ok_or_else(changed)?.dir;
            let current = RetainedFile::open(directory, name, file.metadata.len(), false)?;
            if current.identity != file.identity
                || current.sha256 != file.sha256
                || !platform::same_state(&current.metadata, &file.metadata)
            {
                return Err(changed());
            }
        }
        Ok(())
    }

    fn validate_inventory(&self, key: &str, directory: &Dir) -> Result<(), Diagnostic> {
        let expected = self
            .directories
            .keys()
            .chain(self.files.keys())
            .filter_map(|path| {
                if path.is_empty() {
                    return None;
                }
                let (parent, name) = parent_name(path);
                (parent == key).then_some(name.to_owned())
            })
            .collect::<BTreeSet<_>>();
        let mut actual = BTreeSet::new();
        for entry in directory.entries().map_err(|_| changed())? {
            if actual.len() >= expected.len() {
                return Err(changed());
            }
            let name =
                entry.map_err(|_| changed())?.file_name().into_string().map_err(|_| changed())?;
            if !expected.contains(&name) || !actual.insert(name) {
                return Err(changed());
            }
        }
        if actual == expected { Ok(()) } else { Err(changed()) }
    }
}

fn retained_directory(dir: Dir) -> Result<Directory, Diagnostic> {
    let identity = platform::directory_identity(&dir).map_err(|_| changed())?;
    let metadata = identity.as_file().metadata().map_err(|_| changed())?;
    Ok(Directory { dir, identity, metadata })
}

fn parent_name(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn changed() -> Diagnostic {
    admission_error("installation topology, file identity or bytes are unsafe or changed")
}
