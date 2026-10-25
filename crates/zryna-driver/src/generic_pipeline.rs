//! Ordinary build buffers can leave a NEW execution scope; its authority cannot.
use crate::generic_manifest::{GenericNativeRecord, GenericTarget};
use crate::{CommandFailure, TargetSelection, source_scope::GenericSourceReceipt};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod emit;
mod publish;
pub(crate) use publish::publish;
pub(crate) use publish::publish_run;
pub use publish::{GenericTargetResult, PublishedGenericBundle};

pub(crate) struct PreparedGenericBuild {
    receipt: GenericSourceReceipt,
    artifacts: Vec<OrdinaryArtifact>,
    native: Option<GenericNativeRecord>,
    diagnostics: Vec<zryna_diagnostics::Diagnostic>,
}
struct OrdinaryArtifact {
    target: GenericTarget,
    bytes: Vec<u8>,
}

pub(crate) fn prepare(
    program: &VerifiedExecution<'_>,
    receipt: &GenericSourceReceipt,
    targets: TargetSelection,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<PreparedGenericBuild, CommandFailure> {
    if !receipt.is_bound_to(program) || targets.component() {
        return Err(rejected("ordinary generic build requires its same source receipt and target"));
    }
    checkpoint()?;
    let (artifacts, native) = emit::ordinary(program, targets, checkpoint)?;
    checkpoint()?;
    Ok(PreparedGenericBuild {
        receipt: receipt.clone(),
        artifacts,
        native,
        diagnostics: Vec::new(),
    })
}

impl PreparedGenericBuild {
    pub(crate) fn retain_advisories(&mut self, diagnostics: &[zryna_diagnostics::Diagnostic]) {
        self.diagnostics.extend_from_slice(diagnostics);
    }
}

fn rejected(reason: &'static str) -> CommandFailure {
    CommandFailure {
        kind: crate::CommandFailureKind::Preparation,
        diagnostics: vec![zryna_diagnostics::Diagnostic::error(
            "ZRYNA-C3411",
            None,
            reason,
            "retain the same verified generic source and its ordinary target artifacts",
        )],
    }
}

#[cfg(test)]
mod tests;
