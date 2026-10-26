//! Concrete typed native preparation and actual prepared-identity projection only.
use super::{
    ArtifactOutputRoot, CommandFailure, CommandFailureKind, GenericNativeExecutionRecord,
    GenericRunHosts, failure,
};
use crate::{
    generic_runtime_v1::RuntimeSource,
    native::generic_execution::{self, PreparedGenericExecutable},
};
use zryna_abi::{Invocation, ScalarOutcome};
use zryna_backend_native::{
    ValidatedNativeObjectArtifact, generic_execution_v1::ValidatedInvocationArtifact,
};
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
fn target() -> Result<zryna_backend_native::LinuxX8664ObjectTarget, CommandFailure> {
    zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
        .map_err(failure::preparation)
}
pub(super) fn ordinary(
    mir: &VerifiedProgram<'_, '_>,
) -> Result<ValidatedNativeObjectArtifact, CommandFailure> {
    zryna_backend_native::generic_execution_v1::emit_object(mir, target()?)
        .map_err(failure::preparation)
}
pub(super) fn carrier<'m, 'p, 'a>(
    mir: &'m VerifiedProgram<'p, 'a>,
    request: Invocation,
) -> Result<ValidatedInvocationArtifact<'m, 'p, 'a>, CommandFailure> {
    zryna_backend_native::generic_execution_v1::emit_invocation(mir, request, target()?)
        .map_err(failure::preparation)
}
pub(super) fn prepare<'m, 'p, 'a>(
    source: RuntimeSource<'m, 'p, 'a>,
    carrier: &ValidatedInvocationArtifact<'_, 'p, 'a>,
    hosts: &GenericRunHosts<'_>,
    output: &ArtifactOutputRoot,
) -> Result<PreparedGenericExecutable<'m, 'p, 'a>, CommandFailure> {
    let tools = hosts
        .tools
        .as_ref()
        .ok_or_else(|| failure::boundary("missing captured native toolchain"))?;
    generic_execution::prepare(source, carrier, output, tools, hosts.limits)
        .map_err(|errors| failure::native(errors, CommandFailureKind::Preparation))
}
pub(super) fn record(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    receipt: &crate::source_scope::GenericSourceReceipt,
    request: &zryna_abi::VerifiedInvocation<'_>,
) -> Result<GenericNativeExecutionRecord, CommandFailure> {
    let identity = prepared.identity();
    let material = identity.material().material();
    if !receipt.is_bound_to(prepared.source().program())
        || material.universe_identity() != receipt.type_universe_sha256()
        || material.linear_layout_fingerprint() != receipt.linear32_sha256()
        || material.native_layout_fingerprint() != receipt.linux_x86_64_sha256()
        || material.runtime_abi_fingerprint() != receipt.runtime_abi_sha256()
        || identity.image().required_tls_bytes() != material.required_tls_bytes()
        || identity.entry_symbol() != "zryna_gexecution_invoke_v1"
        || identity.logical_export() != request.export().logical_name().as_str()
        || identity.arguments() != request.arguments()
        || identity.result_type() != request.export().result()
    {
        return Err(failure::boundary(
            "native execution evidence differs from its source and request",
        ));
    }
    let hex = crate::generic_manifest::source::hex;
    Ok(GenericNativeExecutionRecord {
        ordinary_object: hex(identity.material().material().ordinary_object_sha256()),
        runtime_object: hex(identity.material().runtime_object_sha256()),
        invocation_object: hex(identity.invocation_object_sha256()),
        harness: hex(identity.harness_sha256()),
        executable: hex(identity.image().executable_sha256()),
        dependency_receipt: crate::generic_manifest::receipt_sha256(identity.dependencies())
            .map_err(failure::preparation)?,
    })
}
pub(super) fn run(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    hosts: &GenericRunHosts<'_>,
    output: &ArtifactOutputRoot,
) -> Result<ScalarOutcome, CommandFailure> {
    generic_execution::run(prepared, output, hosts.limits)
        .map_err(|errors| failure::native(errors, CommandFailureKind::Execution))
}
