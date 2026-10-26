//! Fixed system GNU provider capture; observed hashes are evidence under the toolchain grant.

mod capture;
mod files;
use crate::native::closure_inventory as closure;

use super::super::{LinuxX8664LinkToolchain, NativeProcessLimits, NativeStage};
pub(super) use closure::{ClosureInventory, ObjectInventory};
use files::{CapturedFile, ToolSeal};
use zryna_diagnostics::Diagnostic;

const INTERPRETER: &str = "/lib64/ld-linux-x86-64.so.2";
pub(super) const LOADER: &str = "ld-linux-x86-64.so.2";
pub(super) const LIBC: &str = "libc.so.6";

pub(super) struct Dependencies {
    tools: ToolSeal,
    loader: CapturedFile,
    libc: CapturedFile,
    identity: DependencyIdentity,
}

/// Observation under the fixed GNU runtime capability, not provider approval from hashes.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct DependencyIdentity {
    driver_sha256: [u8; 32],
    linker_sha256: [u8; 32],
    loader_sha256: [u8; 32],
    libc_sha256: [u8; 32],
    interpreter: &'static str,
    gcc_version: String,
    linker_version: String,
}

impl Dependencies {
    pub(super) fn capture(
        stage: &NativeStage,
        tools: &LinuxX8664LinkToolchain,
        limits: NativeProcessLimits,
    ) -> Result<Self, Diagnostic> {
        capture::issue(stage, tools, limits)
    }

    pub(super) fn revalidate(&self) -> Result<(), Diagnostic> {
        self.tools.revalidate()?;
        self.loader.revalidate()?;
        self.libc.revalidate()?;
        self.loader.revalidate_alias(std::path::Path::new(INTERPRETER))?;
        closure::validate_providers(&self.retained(), &self.expectation()).map_err(|_| rejected())
    }

    pub(super) fn tools(&self) -> &LinuxX8664LinkToolchain {
        &self.tools.tools
    }
    pub(super) fn identity(&self) -> &DependencyIdentity {
        &self.identity
    }
    pub(super) fn materials(&self) -> [(&'static str, &[u8]); 2] {
        [(LOADER, &self.loader.bytes), (LIBC, &self.libc.bytes)]
    }

    pub(super) fn audit(&self, image: &[u8]) -> Result<ClosureInventory, Diagnostic> {
        self.revalidate()?;
        closure::audit(image, &self.retained(), &self.expectation()).map_err(|_| rejected())
    }

    fn retained(&self) -> [closure::RetainedProvider; 2] {
        [
            closure::RetainedProvider { id: LOADER.into(), bytes: self.loader.bytes.clone() },
            closure::RetainedProvider { id: LIBC.into(), bytes: self.libc.bytes.clone() },
        ]
    }

    fn expectation(&self) -> closure::ProviderExpectation {
        closure::ProviderExpectation {
            interpreter: INTERPRETER.into(),
            needed: vec![LIBC.into()],
            providers: vec![
                closure::ProviderRule {
                    id: LOADER.into(),
                    sha256: self.loader.sha256,
                    soname: Some(LOADER.into()),
                    needed: vec![],
                },
                closure::ProviderRule {
                    id: LIBC.into(),
                    sha256: self.libc.sha256,
                    soname: Some(LIBC.into()),
                    needed: vec![LOADER.into()],
                },
            ],
            // Fixed GNU libc/loader classes, never caller-controlled provider permissions.
            allow_weak: true,
            allow_ifunc: true,
            allow_tls: true,
        }
    }
}

pub(super) fn no_preload() -> Result<(), Diagnostic> {
    match std::fs::symlink_metadata("/etc/ld.so.preload") {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(rejected()),
    }
}

fn rejected() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4165",
        "generic native GNU provider capture or closed dependency validation failed",
        "retain the captured documented GNU toolchain, exact libc and loader, no global preload, and the private fixed provider staging directory",
    )
}
