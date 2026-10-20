//! Staging the complete canonical ownership artifact sequence.

use super::{
    Checkpoint, CommandFailure, DataOwnershipCandidateSuccess, OwnershipTarget, PublicationPhase,
    PublishedOwnershipArtifact, Transaction, transaction_failure,
};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn stage_artifacts(
    transaction: &Transaction,
    success: &DataOwnershipCandidateSuccess,
    bundle: &Path,
    checkpoint: Checkpoint<'_>,
) -> Result<Vec<PublishedOwnershipArtifact>, CommandFailure> {
    let mut published = Vec::with_capacity(3);
    let stem = success.artifact_stem();
    let artifacts = success.artifacts();
    if let Some(artifact) = artifacts.javascript() {
        published.push(stage_artifact(
            transaction,
            bundle,
            OwnershipTarget::JavaScript,
            "ecmascript-module",
            stem,
            "mjs",
            artifact.source.as_bytes(),
        )?);
        checkpoint(PublicationPhase::JavaScript)?;
    }
    if let Some(artifact) = artifacts.webassembly() {
        published.push(stage_artifact(
            transaction,
            bundle,
            OwnershipTarget::WebAssembly,
            "core-webassembly-module",
            stem,
            "wasm",
            artifact.bytes(),
        )?);
        checkpoint(PublicationPhase::WebAssembly)?;
    }
    if let Some(artifact) = artifacts.native_object() {
        published.push(stage_artifact(
            transaction,
            bundle,
            OwnershipTarget::Native,
            "linux-x86-64-relocatable-object",
            stem,
            "o",
            artifact.bytes(),
        )?);
        checkpoint(PublicationPhase::Native)?;
    }
    if let Some(artifact) = artifacts.native_executable() {
        published.push(stage_artifact(
            transaction,
            bundle,
            OwnershipTarget::Native,
            "linux-x86-64-invocation-executable",
            stem,
            "elf",
            artifact.executable_bytes(),
        )?);
        checkpoint(PublicationPhase::Native)?;
    }
    if published.is_empty() || published.windows(2).any(|pair| pair[0].target >= pair[1].target) {
        return Err(transaction_failure("ownership artifact order is empty or invalid"));
    }
    Ok(published)
}

fn stage_artifact(
    transaction: &Transaction,
    bundle: &Path,
    target: OwnershipTarget,
    kind: &'static str,
    stem: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<PublishedOwnershipArtifact, CommandFailure> {
    transaction.write_ownership_artifact(target, kind, stem, extension, bytes)?;
    let target_name = match target {
        OwnershipTarget::JavaScript => "javascript",
        OwnershipTarget::WebAssembly => "webassembly",
        OwnershipTarget::Native => "native",
    };
    let path = bundle.join(target_name).join(format!("{stem}.{extension}"));
    Ok(PublishedOwnershipArtifact {
        target,
        path,
        bytes: u64::try_from(bytes.len())
            .map_err(|_| transaction_failure("ownership artifact length exceeds u64"))?,
        sha256: Sha256::digest(bytes).into(),
    })
}
