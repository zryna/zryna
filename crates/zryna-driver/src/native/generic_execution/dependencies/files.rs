//! Stable provider/tool file bytes are retained under the captured GNU toolchain capability.

use super::{no_preload, rejected};
use crate::native::{
    LinuxX8664LinkToolchain, MAX_NATIVE_EXECUTABLE_BYTES, ToolFileIdentity, revalidate_tool,
};
use sha2::{Digest as _, Sha256};
use std::{path::PathBuf, sync::Arc};
use zryna_diagnostics::Diagnostic;

pub(super) struct CapturedFile {
    requested: PathBuf,
    pub(super) canonical: PathBuf,
    identity: ToolFileIdentity,
    pub(super) bytes: Arc<[u8]>,
    pub(super) sha256: [u8; 32],
}

impl CapturedFile {
    pub(super) fn capture(requested: PathBuf) -> Result<Self, Diagnostic> {
        if !requested.is_absolute() {
            return Err(rejected());
        }
        let canonical = std::fs::canonicalize(&requested).map_err(|_| rejected())?;
        let (identity, bytes) =
            super::super::stable::read(&canonical, MAX_NATIVE_EXECUTABLE_BYTES)?;
        let sha256 = Sha256::digest(&bytes).into();
        let result = Self { requested, canonical, identity, sha256, bytes: bytes.into() };
        result.revalidate()?;
        Ok(result)
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        if std::fs::canonicalize(&self.requested).map_err(|_| rejected())? != self.canonical {
            return Err(rejected());
        }
        let (identity, bytes) =
            super::super::stable::read(&self.canonical, MAX_NATIVE_EXECUTABLE_BYTES)?;
        if identity != self.identity || <[u8; 32]>::from(Sha256::digest(&bytes)) != self.sha256 {
            return Err(rejected());
        }
        Ok(())
    }

    pub(super) fn revalidate_alias(&self, alias: &std::path::Path) -> Result<(), Diagnostic> {
        let canonical = std::fs::canonicalize(alias).map_err(|_| rejected())?;
        let (identity, bytes) =
            super::super::stable::read(&canonical, MAX_NATIVE_EXECUTABLE_BYTES)?;
        if identity != self.identity || <[u8; 32]>::from(Sha256::digest(bytes)) != self.sha256 {
            return Err(rejected());
        }
        Ok(())
    }
}

pub(super) struct ToolSeal {
    pub(super) tools: LinuxX8664LinkToolchain,
    pub(super) driver_sha256: [u8; 32],
    pub(super) linker_sha256: [u8; 32],
}

impl ToolSeal {
    pub(super) fn capture(tools: &LinuxX8664LinkToolchain) -> Result<Self, Diagnostic> {
        no_preload()?;
        revalidate_tool(&tools.driver, &tools.driver_identity)?;
        revalidate_tool(&tools.linker, &tools.linker_identity)?;
        let (driver_id, driver) =
            super::super::stable::read(&tools.driver, MAX_NATIVE_EXECUTABLE_BYTES)?;
        let (linker_id, linker) =
            super::super::stable::read(&tools.linker, MAX_NATIVE_EXECUTABLE_BYTES)?;
        if driver_id != tools.driver_identity || linker_id != tools.linker_identity {
            return Err(rejected());
        }
        Ok(Self {
            tools: tools.clone(),
            driver_sha256: Sha256::digest(driver).into(),
            linker_sha256: Sha256::digest(linker).into(),
        })
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        let actual = Self::capture(&self.tools)?;
        if actual.driver_sha256 != self.driver_sha256 || actual.linker_sha256 != self.linker_sha256
        {
            return Err(rejected());
        }
        Ok(())
    }
}
