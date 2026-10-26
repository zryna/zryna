//! Exact copied provider inputs use the ordinary stage writer's exclusion and durability rules.

use super::*;
use cap_std::fs::OpenOptionsExt as _;
use sha2::{Digest as _, Sha256};
use std::{io::Write as _, os::unix::fs::PermissionsExt as _};

impl ProviderStage<'_> {
    pub(super) fn write(&self, name: &str, bytes: &[u8]) -> Result<ToolFileIdentity, Diagnostic> {
        self.directory_identity()?;
        if ![super::super::dependencies::LOADER, super::super::dependencies::LIBC].contains(&name)
            || bytes.is_empty()
            || bytes.len() > crate::native::MAX_NATIVE_EXECUTABLE_BYTES
        {
            return Err(rejected());
        }
        let _writer = crate::process_spawn::snapshot_writer().map_err(|_| rejected())?;
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true).mode(0o600);
        let mut file = self.handle.open_with(name, &options).map_err(|_| rejected())?;
        file.write_all(bytes)
            .and_then(|()| file.flush())
            .and_then(|()| file.sync_all())
            .map_err(|_| rejected())?;
        let mode = if name == super::super::dependencies::LOADER { 0o700 } else { 0o600 };
        file.set_permissions(cap_std::fs::Permissions::from_std(fs::Permissions::from_mode(mode)))
            .map_err(|_| rejected())?;
        file.sync_all().map_err(|_| rejected())?;
        drop(file);
        self.directory_identity()?;
        let (identity, actual) = super::super::stable::read(
            &self.directory().join(name),
            crate::native::MAX_NATIVE_EXECUTABLE_BYTES,
        )?;
        if actual != bytes {
            return Err(rejected());
        }
        Ok(identity)
    }

    pub(in super::super) fn revalidate(
        &self,
        dependencies: &Dependencies,
    ) -> Result<(), Diagnostic> {
        self.directory_identity()?;
        let mut names = Vec::new();
        names.try_reserve_exact(2).map_err(|_| rejected())?;
        for entry in self.handle.entries().map_err(|_| rejected())? {
            if names.len() == 2 {
                return Err(rejected());
            }
            names.push(entry.map_err(|_| rejected())?.file_name());
        }
        names.sort();
        let mut expected = dependencies.materials().map(|(name, _)| std::ffi::OsString::from(name));
        expected.sort();
        if names.as_slice() != expected.as_slice() || self.files.len() != 2 {
            return Err(rejected());
        }
        for (name, original) in dependencies.materials() {
            let path = self.directory().join(name);
            let metadata = fs::symlink_metadata(&path).map_err(|_| rejected())?;
            let mode = if name == super::super::dependencies::LOADER { 0o700 } else { 0o600 };
            if metadata.permissions().mode() & 0o777 != mode {
                return Err(rejected());
            }
            let (identity, bytes) =
                super::super::stable::read(&path, crate::native::MAX_NATIVE_EXECUTABLE_BYTES)?;
            if self
                .files
                .iter()
                .find(|(file, _)| *file == name)
                .is_none_or(|(_, expected)| *expected != identity)
                || bytes.len() != original.len()
                || Sha256::digest(&bytes) != Sha256::digest(original)
            {
                return Err(rejected());
            }
        }
        self.directory_identity()
    }
}
