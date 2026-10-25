//! Typed Node execution keeps the same pre/post source and output checkpoint.
use super::{
    ArtifactOutputRoot, CommandFailure, GenericObservedOutcome, GenericRunHosts, GenericTarget,
};

pub(super) fn javascript(
    carrier: &zryna_backend_javascript::generic_execution_v1::ValidatedInvocationArtifact<'_, '_>,
    hosts: &GenericRunHosts<'_>,
    output: &ArtifactOutputRoot,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<GenericObservedOutcome, CommandFailure> {
    checkpoint()?;
    let outcome = hosts
        .node()?
        .run_generic_javascript(carrier, output.path())
        .map_err(crate::pipeline::runtime_failure)?;
    checkpoint()?;
    Ok(GenericObservedOutcome { target: GenericTarget::JavaScript, outcome })
}

pub(super) fn webassembly(
    carrier: &zryna_backend_webassembly::generic_execution_v1::ValidatedInvocationArtifact<'_, '_>,
    hosts: &GenericRunHosts<'_>,
    output: &ArtifactOutputRoot,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<GenericObservedOutcome, CommandFailure> {
    checkpoint()?;
    let outcome = hosts
        .node()?
        .run_generic_webassembly(carrier, output.path())
        .map_err(crate::pipeline::runtime_failure)?;
    checkpoint()?;
    Ok(GenericObservedOutcome { target: GenericTarget::WebAssembly, outcome })
}
