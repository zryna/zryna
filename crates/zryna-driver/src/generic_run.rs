//! Only completed typed host executions issue the private owned run receipt.
use crate::{
    ArtifactOutputRoot, CommandFailure, CommandFailureKind, TargetSelection,
    generic_manifest::{
        GenericInvocationRecord, GenericNativeExecutionRecord, GenericObservedOutcome,
        GenericTarget,
    },
    runtime::NodeRuntimeCapability,
    source_scope::GenericSourceReceipt,
};
use zryna_abi::{Invocation, ScalarValue, VerifiedInvocation};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod failure;
mod hosts;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod native;
mod node;
pub(crate) use hosts::GenericRunHosts;

/// Owned completion cannot be reconstructed from manifest wire metadata.
pub(crate) struct CompletedGenericRun {
    source: GenericSourceReceipt,
    invocation: GenericInvocationRecord,
    native: Option<GenericNativeExecutionRecord>,
}
impl CompletedGenericRun {
    pub(crate) fn into_records(
        self,
        expected: &GenericSourceReceipt,
    ) -> Result<(GenericInvocationRecord, Option<GenericNativeExecutionRecord>), CommandFailure>
    {
        if &self.source != expected {
            return Err(failure::boundary("completed run belongs to another source world"));
        }
        Ok((self.invocation, self.native))
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the scoped command boundary retains each independent source/host/output capability explicitly"
)]
pub(crate) fn execute(
    program: &VerifiedExecution<'_>,
    receipt: &GenericSourceReceipt,
    targets: TargetSelection,
    hosts: &GenericRunHosts<'_>,
    output: &ArtifactOutputRoot,
    export: String,
    arguments: Vec<ScalarValue>,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<CompletedGenericRun, CommandFailure> {
    let request = validate(program, receipt, export, arguments)?;
    hosts.validate(targets)?;
    let check = || {
        checkpoint()?;
        output.revalidate().map_err(failure::preparation)
    };
    check()?;
    let javascript = if targets.javascript() {
        Some(
            zryna_backend_javascript::generic_execution_v1::emit_invocation(
                program,
                copy_request(&request),
            )
            .map_err(failure::preparation)?,
        )
    } else {
        None
    };
    let webassembly = if targets.webassembly() {
        Some(
            zryna_backend_webassembly::generic_execution_v1::emit_invocation(
                program,
                copy_request(&request),
            )
            .map_err(failure::preparation)?,
        )
    } else {
        None
    };
    // Distinct stack owners retain every native borrow until all executions have finished.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let mir = if targets.native() {
        Some(zryna_native_mir::generic_execution_v1::lower(program).map_err(failure::preparation)?)
    } else {
        None
    };
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let ordinary = mir.as_ref().map(native::ordinary).transpose()?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let carrier =
        mir.as_ref().map(|mir| native::carrier(mir, copy_request(&request))).transpose()?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let material = match (mir.as_ref(), ordinary.as_ref()) {
        (Some(mir), Some(object)) => Some(
            crate::generic_runtime_v1::RuntimeSource::issue(program, mir, object)
                .map_err(failure::preparation)?,
        ),
        (None, None) => None,
        _ => return Err(failure::boundary("incomplete ordinary native preparation")),
    };
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let prepared = match (material, carrier.as_ref()) {
        (Some(source), Some(carrier)) => Some(native::prepare(source, carrier, hosts, output)?),
        (None, None) => None,
        _ => return Err(failure::boundary("incomplete invocation native preparation")),
    };
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    if targets.native() {
        return Err(failure::boundary("native execution requires the captured Linux x86-64 host"));
    }
    check()?;
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(3)
        .map_err(|_| failure::boundary("run result reservation failed"))?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let native_record = prepared
        .as_ref()
        .map(|prepared| native::record(prepared, receipt, &request))
        .transpose()?;
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let native_record = None;
    // Every selected typed artifact/image is now ready. No guest has run above this line.
    if let Some(carrier) = &javascript {
        outcomes.push(node::javascript(carrier, hosts, output, &check)?);
    }
    if let Some(carrier) = &webassembly {
        outcomes.push(node::webassembly(carrier, hosts, output, &check)?);
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if let Some(prepared) = &prepared {
        check()?;
        let outcome = native::run(prepared, hosts, output)?;
        check()?;
        outcomes.push(GenericObservedOutcome { target: GenericTarget::Native, outcome });
    }
    Ok(CompletedGenericRun {
        source: receipt.clone(),
        invocation: GenericInvocationRecord {
            export: request.export().logical_name().as_str().to_owned(),
            arguments: request.arguments().to_vec(),
            outcomes,
        },
        native: native_record,
    })
}
fn validate<'p>(
    program: &'p VerifiedExecution<'_>,
    receipt: &GenericSourceReceipt,
    export: String,
    arguments: Vec<ScalarValue>,
) -> Result<VerifiedInvocation<'p>, CommandFailure> {
    if !receipt.is_bound_to(program) {
        return Err(failure::boundary("run receipt differs from the exact source execution"));
    }
    program.scalar_abi().prepare_invocation(Invocation::new(export, arguments)).map_err(|error| {
        CommandFailure {
            kind: CommandFailureKind::Request,
            diagnostics: vec![zryna_diagnostics::Diagnostic::error(
                error.code(),
                None,
                "generic invocation does not match the selected scalar source export",
                "use the exact source export and typed scalar arguments",
            )],
        }
    })
}
fn copy_request(request: &VerifiedInvocation<'_>) -> Invocation {
    Invocation::new(
        request.export().logical_name().as_str().to_owned(),
        request.arguments().to_vec(),
    )
}
#[cfg(test)]
mod tests;
