//! Compiler-owned native provider selection, retaining the currently executing image.

use std::{
    ffi::OsString,
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use same_file::Handle;
use sha2::{Digest as _, Sha256};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, ProviderExpectationV3, ProviderExpectationV4,
    WorkerFrontend, WorkerFrontendV3, WorkerFrontendV4, WorkerLimits, WorkerLimitsV3,
    WorkerLimitsV4, WorkerSpec, WorkerSpecV3, WorkerSpecV4,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
};

use crate::runtime::{open_runtime_identity, same_file_state, stable_invocation_path};

mod comparison;
mod comparison_v5;
mod v5;
pub use comparison::{ComparisonFrontendV2, ComparisonFrontendV3, ComparisonFrontendV4};
pub(crate) use comparison_v5::ComparisonFrontendV5;

const MAX_COMPILER_BYTES: u64 = 512 * 1024 * 1024;

/// The actual running compiler image retained for native-worker invocation.
///
/// No constructor accepts a caller-selected executable or an adjacent identity document.
/// Keep this capability alive and revalidate it at analysis/publication checkpoints.
#[derive(Debug)]
pub struct NativeProviderCapability {
    executable: PathBuf,
    invocation: PathBuf,
    directory: PathBuf,
    identity: Handle,
    state: fs::Metadata,
    sha256: String,
}

impl NativeProviderCapability {
    /// Captures the current executable using the existing no-follow runtime identity strategy.
    ///
    /// # Errors
    /// Rejects unsupported platforms, unstable/nonordinary images and compiler byte overflow.
    pub fn capture_current() -> Result<Self, Diagnostic> {
        let executable = std::env::current_exe().map_err(|_| identity_error())?;
        let (identity, state) = open_runtime_identity(&executable).map_err(|_| identity_error())?;
        if state.len() == 0 || state.len() > MAX_COMPILER_BYTES {
            return Err(identity_error());
        }
        let invocation =
            stable_invocation_path(&executable, &identity).map_err(|_| identity_error())?;
        let directory = executable.parent().ok_or_else(identity_error)?.to_path_buf();
        let mut file = identity.as_file().try_clone().map_err(|_| identity_error())?;
        file.seek(SeekFrom::Start(0)).map_err(|_| identity_error())?;
        let mut digest = Sha256::new();
        let mut reader = file.take(MAX_COMPILER_BYTES + 1);
        let mut bytes = 0_u64;
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buffer).map_err(|_| identity_error())?;
            if n == 0 {
                break;
            }
            bytes = bytes.checked_add(n as u64).ok_or_else(identity_error)?;
            if bytes > MAX_COMPILER_BYTES {
                return Err(identity_error());
            }
            digest.update(&buffer[..n]);
        }
        if bytes != state.len() {
            return Err(identity_error());
        }
        let capability = Self {
            executable,
            invocation,
            directory,
            identity,
            state,
            sha256: format!("{:x}", digest.finalize()),
        };
        capability.revalidate()?;
        Ok(capability)
    }

    /// Revalidates both the retained image and its current named identity.
    ///
    /// Linux invokes the retained file descriptor; Windows retains the existing read-only
    /// non-write/non-delete-sharing image handle for the complete owner lifetime.
    ///
    /// # Errors
    /// Rejects replacement, mutation or loss of the original executable before dispatch.
    pub fn revalidate(&self) -> Result<(), Diagnostic> {
        let held = self.identity.as_file().metadata().map_err(|_| identity_error())?;
        let (named, state) =
            open_runtime_identity(&self.executable).map_err(|_| identity_error())?;
        if named != self.identity
            || !same_file_state(&self.state, &held)
            || !same_file_state(&self.state, &state)
        {
            return Err(identity_error());
        }
        Ok(())
    }

    /// Returns the SHA-256 of the complete captured compiler image.
    #[must_use]
    pub fn compiler_sha256(&self) -> &str {
        &self.sha256
    }

    /// Creates the original protocol-v2 worker with exact native identity and hard limits.
    ///
    /// # Errors
    /// Rejects stale retained executable identity or invalid original worker configuration.
    pub fn frontend_v2(&self) -> Result<WorkerFrontend, Diagnostic> {
        self.revalidate()?;
        let expected = ProviderExpectation::new(
            NATIVE_PROVIDER,
            NATIVE_PROVIDER_VERSION,
            2,
            FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
        )
        .map_err(|_| identity_error())?;
        let spec = WorkerSpec::new(
            self.invocation.clone(),
            arguments(2),
            self.directory.clone(),
            expected,
            WorkerLimits::default(),
        )
        .map_err(|_| identity_error())?;
        Ok(WorkerFrontend::new(spec))
    }

    /// Creates the original protocol-v3 worker with exact native identity and hard limits.
    ///
    /// # Errors
    /// Rejects stale retained executable identity or invalid original worker configuration.
    pub fn frontend_v3(&self) -> Result<WorkerFrontendV3, Diagnostic> {
        self.revalidate()?;
        let expected = ProviderExpectationV3::new(NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION)
            .map_err(|_| identity_error())?;
        let spec = WorkerSpecV3::new(
            self.invocation.clone(),
            arguments(3),
            self.directory.clone(),
            expected,
            WorkerLimitsV3::default(),
        )
        .map_err(|_| identity_error())?;
        Ok(WorkerFrontendV3::new(spec))
    }

    /// Creates the original protocol-v4 worker with exact native identity and hard limits.
    ///
    /// # Errors
    /// Rejects stale retained executable identity or invalid original worker configuration.
    pub fn frontend_v4(&self) -> Result<WorkerFrontendV4, Diagnostic> {
        self.revalidate()?;
        let expected = ProviderExpectationV4::new(NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION)
            .map_err(|_| identity_error())?;
        let spec = WorkerSpecV4::new(
            self.invocation.clone(),
            arguments(4),
            self.directory.clone(),
            expected,
            WorkerLimitsV4::default(),
        )
        .map_err(|_| identity_error())?;
        Ok(WorkerFrontendV4::new(spec))
    }
}

fn arguments(protocol: u32) -> Vec<OsString> {
    vec![
        OsString::from("__native-frontend"),
        OsString::from("--protocol"),
        OsString::from(protocol.to_string()),
    ]
}

fn identity_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-F1101",
        None,
        "the running native compiler image could not be retained or changed after admission",
        "use the verified compiler installation and stop concurrent executable replacement",
    )
}

#[cfg(test)]
mod tests;
