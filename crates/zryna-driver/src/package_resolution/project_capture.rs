//! Resolver-produced root bytes and retained graph filesystem identities.

use std::cell::RefCell;

use zryna_package::{PackageFile, ResolveError};

use super::FilesystemProvider;

pub(crate) struct CapturedProject {
    provider: RefCell<FilesystemProvider>,
    root_files: Vec<PackageFile>,
}

impl CapturedProject {
    pub(super) fn new(provider: FilesystemProvider, root_files: Vec<PackageFile>) -> Self {
        Self { provider: RefCell::new(provider), root_files }
    }

    pub(crate) fn root_file(&self, path: &str) -> Option<&[u8]> {
        self.root_files.iter().find(|file| file.path == path).map(|file| file.bytes.as_slice())
    }

    pub(crate) fn revalidate(&self) -> Result<(), ResolveError> {
        self.provider
            .try_borrow_mut()
            .map_err(|_| ResolveError::source("project source revalidation is already active"))?
            .revalidate()
    }

    pub(super) fn into_root_files(self) -> Vec<PackageFile> {
        self.root_files
    }
}
