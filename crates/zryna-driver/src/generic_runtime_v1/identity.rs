//! Owned evidence records have no deserializer or authority constructor.

use sha2::{Digest as _, Sha256};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;

/// Immutable byte identities observed only after same-owner material issuance.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct RuntimeMaterialIdentity {
    template_sha256: [u8; 32],
    header_sha256: [u8; 32],
    rendered_sha256: [u8; 32],
    ordinary_object_sha256: [u8; 32],
    universe_identity: [u8; 32],
    linear_layout_fingerprint: [u8; 32],
    native_layout_fingerprint: [u8; 32],
    runtime_abi_fingerprint: [u8; 32],
    required_tls_bytes: u64,
    target: &'static str,
}

impl RuntimeMaterialIdentity {
    pub(super) fn new(
        program: &VerifiedExecution<'_>,
        mir: &VerifiedProgram<'_, '_>,
        object: &[u8],
        rendered: &[u8],
    ) -> Self {
        let linear = program.layouts(StorageTarget::Linear32V1);
        let native = program.layouts(StorageTarget::LinuxX8664V1);
        Self {
            template_sha256: super::source::template_sha256(),
            header_sha256: digest(super::source::HEADER),
            rendered_sha256: digest(rendered),
            ordinary_object_sha256: digest(object),
            universe_identity: *native.universe_identity(),
            linear_layout_fingerprint: *linear.fingerprint(),
            native_layout_fingerprint: *native.fingerprint(),
            runtime_abi_fingerprint: program.runtime().identity().fingerprint(),
            required_tls_bytes: if mir.requires_recursive_arena() {
                zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS * 32 + 16
            } else {
                0
            },
            target: zryna_backend_native::NATIVE_OBJECT_TARGET,
        }
    }

    pub(crate) const fn template_sha256(&self) -> &[u8; 32] {
        &self.template_sha256
    }
    pub(crate) const fn header_sha256(&self) -> &[u8; 32] {
        &self.header_sha256
    }
    pub(crate) const fn rendered_sha256(&self) -> &[u8; 32] {
        &self.rendered_sha256
    }
    pub(crate) const fn ordinary_object_sha256(&self) -> &[u8; 32] {
        &self.ordinary_object_sha256
    }
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn universe_identity(&self) -> &[u8; 32] {
        &self.universe_identity
    }
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn linear_layout_fingerprint(&self) -> &[u8; 32] {
        &self.linear_layout_fingerprint
    }
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn native_layout_fingerprint(&self) -> &[u8; 32] {
        &self.native_layout_fingerprint
    }
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn runtime_abi_fingerprint(&self) -> &[u8; 32] {
        &self.runtime_abi_fingerprint
    }
    pub(crate) const fn required_tls_bytes(&self) -> u64 {
        self.required_tls_bytes
    }
    pub(crate) const fn target(&self) -> &'static str {
        self.target
    }
}

pub(super) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
