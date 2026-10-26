//! Separate NEW-owner generic native runtime and invocation preparation.
//! No old runtime, MIR, foreign-library fixture or scalar adapter seal is accepted.

mod runtime_audit;
mod wire;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod dependencies;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod harness;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod identity;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod image_audit;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod prepare;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod process;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod provider_stage;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod run;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod stable;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use identity::PreparedIdentity;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use prepare::prepare;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use run::run;
pub(crate) use runtime_audit::{RuntimeObjectIdentity, audit_runtime_object};

/// Prepared only by compiling issued source and auditing the typed standalone invoker image.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) struct PreparedGenericExecutable<'m, 'p, 'a> {
    source: crate::generic_runtime_v1::RuntimeSource<'m, 'p, 'a>,
    bytes: Vec<u8>,
    identity: PreparedIdentity,
    dependencies: dependencies::Dependencies,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl<'m, 'p, 'a> PreparedGenericExecutable<'m, 'p, 'a> {
    pub(crate) const fn source(&self) -> &crate::generic_runtime_v1::RuntimeSource<'m, 'p, 'a> {
        &self.source
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) const fn identity(&self) -> &PreparedIdentity {
        &self.identity
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn staging_error() -> zryna_diagnostics::Diagnostic {
    super::native_error(
        "ZRYNA-C4166",
        "generic native private staging bytes or retained input identity changed or could not be prepared",
        "retain the compiler-owned input, exact bounded file bytes and private stage identities without replacement or extra entries",
    )
}
