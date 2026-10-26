//! The sole prepared-capability issuer compiles only retained source and links typed carrier bytes.

use super::{
    PreparedGenericExecutable, audit_runtime_object, dependencies::Dependencies, harness,
    identity::PreparedIdentity, image_audit, process::Context, staging_error as rejected,
};
use crate::{
    generic_runtime_v1::RuntimeSource,
    native::{
        ArtifactOutputRoot, LinuxX8664LinkToolchain, NativeProcessLimits, NativeStage,
        ensure_linux_x86_64_host,
    },
};
use sha2::{Digest as _, Sha256};
use zryna_backend_native::generic_execution_v1::ValidatedInvocationArtifact;
use zryna_diagnostics::Diagnostic;

pub(crate) fn prepare<'m, 'p, 'a>(
    source: RuntimeSource<'m, 'p, 'a>,
    carrier: &ValidatedInvocationArtifact<'_, 'p, 'a>,
    output: &ArtifactOutputRoot,
    tools: &LinuxX8664LinkToolchain,
    limits: NativeProcessLimits,
) -> Result<PreparedGenericExecutable<'m, 'p, 'a>, Vec<Diagnostic>> {
    ensure_linux_x86_64_host().map_err(|error| vec![error])?;
    if !std::ptr::eq(source.program(), carrier.source())
        || !std::ptr::eq(source.mir(), carrier.mir())
        || carrier.entry_symbol() != "zryna_gexecution_invoke_v1"
    {
        return Err(vec![rejected()]);
    }
    carrier.audit_bytes(carrier.bytes()).map_err(|error| vec![error])?;
    let harness =
        harness::render(carrier.invocation().export().result()).map_err(|error| vec![error])?;
    let stage = NativeStage::create(output, "generic-invocation").map_err(|error| vec![error])?;
    let result: Result<_, Vec<Diagnostic>> = (|| {
        stage.write_input(&stage.object, carrier.bytes()).map_err(|error| vec![error])?;
        stage.write_input(&stage.runtime_source, source.bytes()).map_err(|error| vec![error])?;
        stage.write_input(&stage.harness, &harness).map_err(|error| vec![error])?;
        let dependencies =
            Dependencies::capture(&stage, tools, limits).map_err(|error| vec![error])?;
        let mut context = Context::capture(
            &stage,
            &dependencies,
            limits,
            super::process::ExpectedInputs {
                program: carrier.bytes(),
                runtime: source.bytes(),
                harness: &harness,
            },
        )
        .map_err(|error| vec![error])?;
        let runtime = context.compile_runtime()?;
        let material = audit_runtime_object(&source, &runtime).map_err(|error| vec![error])?;
        let bytes = context.link()?;
        let closure = dependencies.audit(&bytes).map_err(|error| vec![error])?;
        let image = image_audit::check(&source, &bytes, closure).map_err(|error| vec![error])?;
        let identity = PreparedIdentity::new(
            material,
            Sha256::digest(carrier.bytes()).into(),
            Sha256::digest(&harness).into(),
            image,
            dependencies.identity().clone(),
            carrier.invocation(),
        )
        .map_err(|error| vec![error])?;
        Ok(PreparedGenericExecutable { source, bytes, identity, dependencies })
    })();
    let cleanup = stage.cleanup();
    match result {
        Ok(prepared) if cleanup.is_empty() => Ok(prepared),
        Ok(_) => Err(cleanup),
        Err(mut errors) => {
            errors.extend(cleanup);
            Err(errors)
        }
    }
}
