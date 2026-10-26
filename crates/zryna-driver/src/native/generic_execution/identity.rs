//! Prepared invocation evidence is retained by its opaque NEW-owner capability.

use super::{RuntimeObjectIdentity, dependencies::DependencyIdentity, image_audit::ImageIdentity};
use zryna_abi::{ScalarType, ScalarValue, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;

#[derive(Debug, serde::Serialize)]
pub(crate) struct PreparedIdentity {
    material: RuntimeObjectIdentity,
    invocation_object_sha256: [u8; 32],
    harness_sha256: [u8; 32],
    image: ImageIdentity,
    dependencies: DependencyIdentity,
    logical_export: String,
    arguments: Vec<ScalarValue>,
    result_type: ScalarType,
    entry_symbol: &'static str,
}

impl PreparedIdentity {
    pub(super) fn new(
        material: RuntimeObjectIdentity,
        invocation_object_sha256: [u8; 32],
        harness_sha256: [u8; 32],
        image: ImageIdentity,
        dependencies: DependencyIdentity,
        invocation: &VerifiedInvocation<'_>,
    ) -> Result<Self, Diagnostic> {
        let mut arguments = Vec::new();
        arguments
            .try_reserve_exact(invocation.arguments().len())
            .map_err(|_| super::staging_error())?;
        arguments.extend_from_slice(invocation.arguments());
        Ok(Self {
            material,
            invocation_object_sha256,
            harness_sha256,
            image,
            dependencies,
            logical_export: invocation.export().logical_name().as_str().into(),
            arguments,
            result_type: invocation.export().result(),
            entry_symbol: "zryna_gexecution_invoke_v1",
        })
    }

    pub(crate) const fn material(&self) -> &RuntimeObjectIdentity {
        &self.material
    }
    pub(crate) const fn invocation_object_sha256(&self) -> &[u8; 32] {
        &self.invocation_object_sha256
    }
    pub(crate) const fn harness_sha256(&self) -> &[u8; 32] {
        &self.harness_sha256
    }
    pub(crate) const fn image(&self) -> &ImageIdentity {
        &self.image
    }
    pub(crate) const fn dependencies(&self) -> &DependencyIdentity {
        &self.dependencies
    }
    pub(crate) fn logical_export(&self) -> &str {
        &self.logical_export
    }
    pub(crate) fn arguments(&self) -> &[ScalarValue] {
        &self.arguments
    }
    pub(crate) const fn result_type(&self) -> ScalarType {
        self.result_type
    }
    pub(crate) const fn entry_symbol(&self) -> &'static str {
        self.entry_symbol
    }
}
