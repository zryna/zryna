//! Shared sealed ownership invocation and target execution.

use super::{execution_diagnostic, execution_failure, observation};
use crate::{
    ArtifactOutputRoot, CommandFailure, NativeProcessLimits, OwnershipManifestResult,
    OwnershipTarget, native::ownership::observation::run,
    ownership_pipeline::DataOwnershipCandidateSuccess, pipeline::Transaction,
    runtime::NodeRuntimeCapability,
};
use zryna_abi::{Invocation, VerifiedInvocation};

pub(super) struct PreparedRun<'program> {
    success: &'program DataOwnershipCandidateSuccess,
    invocation: VerifiedInvocation<'program>,
    fault: Option<observation::Fault>,
}

impl<'program> PreparedRun<'program> {
    pub(super) fn new(
        success: &'program DataOwnershipCandidateSuccess,
        fault: Option<observation::Fault>,
    ) -> Result<Self, CommandFailure> {
        let export = success.logical_export().ok_or_else(|| {
            execution_failure(
                "ZRYNA-C3301",
                "candidate run lost its authenticated invocation identity",
            )
        })?;
        let invocation = success
            .program()
            .verified_ir()
            .scalar_abi()
            .prepare_invocation(Invocation::new(export.to_owned(), success.arguments().to_vec()))
            .map_err(|error| {
                execution_failure(
                    error.code(),
                    "candidate run invocation no longer matches its verified scalar ABI",
                )
            })?;
        Ok(Self { success, invocation, fault })
    }

    pub(super) fn execute(
        &self,
        node: &NodeRuntimeCapability,
        transaction: &Transaction,
        output: &ArtifactOutputRoot,
        checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<Vec<OwnershipManifestResult>, CommandFailure> {
        checkpoint()?;
        let mut results = Vec::with_capacity(3);
        let artifacts = self.success.artifacts();
        if artifacts.javascript().is_some() {
            checkpoint()?;
            let harness = observation::javascript(
                self.success.artifact_stem(),
                &self.invocation,
                self.fault,
            )?;
            let harness_path = transaction.write_runtime_harness("javascript", &harness)?;
            let frame = node
                .run_ownership_javascript(&harness_path, transaction.path())
                .map_err(execution_diagnostic)?;
            results.push(observation::decode(
                &frame,
                self.invocation.export().result(),
                OwnershipTarget::JavaScript,
            )?);
        }
        if let Some(artifact) = artifacts.webassembly() {
            checkpoint()?;
            let harness = observation::webassembly(&self.invocation, self.fault)?;
            let frame = node
                .run_ownership_webassembly(&harness, artifact.bytes(), transaction.path())
                .map_err(execution_diagnostic)?;
            results.push(observation::decode(
                &frame,
                self.invocation.export().result(),
                OwnershipTarget::WebAssembly,
            )?);
        }
        if let Some(executable) = artifacts.native_executable() {
            checkpoint()?;
            let frame = run(
                executable.prepared_executable(),
                output,
                NativeProcessLimits::default(),
                self.fault.map(observation::Fault::command),
            )
            .map_err(|error| execution_diagnostic(error.diagnostic().clone()))?;
            results.push(observation::decode(
                &frame,
                self.invocation.export().result(),
                OwnershipTarget::Native,
            )?);
        }
        node.revalidate().map_err(execution_diagnostic)?;
        checkpoint()?;
        Ok(results)
    }
}
