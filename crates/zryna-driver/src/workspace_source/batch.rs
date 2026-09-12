//! Provider batches retain exact source authority without rereading the discovered prefix.

use std::collections::{BTreeMap, BTreeSet};

use cap_fs_ext::DirExt as _;
use zryna_diagnostics::Diagnostic;
use zryna_source::SourceMap;

use super::{
    DirectoryIndex, WorkspaceSourceSession, changed, changed_path, directory_identity_and_metadata,
    metadata_is_link_or_reparse, root_child_error, same_file_state, scan_entries, unsafe_root,
};

impl WorkspaceSourceSession<'_> {
    pub(crate) fn validate_provider_batch(
        &mut self,
        sources: &SourceMap,
    ) -> Result<(), Diagnostic> {
        let mut paths = Vec::with_capacity(sources.len());
        let mut directories = BTreeSet::from([String::new()]);
        for index in 0..sources.len() {
            let raw = u32::try_from(index).map_err(|_| unsafe_root())?;
            let id = sources.verify_file_id(raw).map_err(|_| unsafe_root())?;
            let input = sources.source(id).ok_or_else(unsafe_root)?;
            let retained = self.sources.get(input.path()).ok_or_else(|| changed(input.path()))?;
            if input.text().as_bytes() != retained.stable.text.as_bytes() {
                return Err(changed(input.path()));
            }
            paths.push(input.path().clone());
            let mut key = retained.parent.clone();
            while directories.insert(key.clone()) {
                let directory = self.directories.get(&key).ok_or_else(unsafe_root)?;
                key = directory.parent.clone().ok_or_else(unsafe_root)?;
            }
        }
        let indexes = self.revalidate_directories(&directories)?;
        for path in paths {
            self.revalidate_source_with_indexes(&path, &indexes)?;
        }
        Ok(())
    }

    pub(crate) fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        let directories = self.directories.keys().cloned().collect();
        let indexes = self.revalidate_directories(&directories)?;
        let paths = self.sources.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            self.revalidate_source_with_indexes(&path, &indexes)?;
        }
        Ok(())
    }

    fn revalidate_directories(
        &self,
        keys: &BTreeSet<String>,
    ) -> Result<BTreeMap<String, DirectoryIndex>, Diagnostic> {
        let mut indexes = BTreeMap::new();
        for key in keys {
            let directory = self.directories.get(key).ok_or_else(unsafe_root)?;
            indexes.insert(key.clone(), scan_entries(&directory.dir).map_err(root_child_error)?);
        }
        self.revalidate_root()?;
        for key in keys.iter().filter(|key| !key.is_empty()) {
            let expected = self.directories.get(key).ok_or_else(unsafe_root)?;
            let parent_key = expected.parent.as_ref().ok_or_else(unsafe_root)?;
            let name = expected.name.as_deref().ok_or_else(unsafe_root)?;
            indexes
                .get(parent_key)
                .ok_or_else(unsafe_root)?
                .require_exact(name)
                .map_err(|_| changed_path(key))?;
            let parent = self.directories.get(parent_key).ok_or_else(unsafe_root)?;
            let current = parent.dir.open_dir_nofollow(name).map_err(|_| changed_path(key))?;
            let (identity, metadata) =
                directory_identity_and_metadata(&current).map_err(|_| changed_path(key))?;
            if identity != expected.identity
                || !same_file_state(&metadata, &expected.metadata)
                || !metadata.is_dir()
                || metadata_is_link_or_reparse(&metadata)
            {
                return Err(changed_path(key));
            }
        }
        Ok(indexes)
    }
}
