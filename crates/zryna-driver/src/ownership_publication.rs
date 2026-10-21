//! Create-only atomic publication of complete candidate ownership bundles.

use std::path::{Path, PathBuf};

mod staging;
use staging::stage_artifacts;
use zryna_diagnostics::Diagnostic;

use crate::{
    ArtifactOutputRoot, CommandFailure, CommandFailureKind, CommandKind, OWNERSHIP_MANIFEST_NAME,
    OwnershipManifestResult, OwnershipTarget, decode_ownership_manifest_v3,
    ownership_manifest::render_ownership_manifest_v3,
    ownership_pipeline::DataOwnershipCandidateSuccess, pipeline::Transaction,
};

/// One published artifact in canonical target order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedOwnershipArtifact {
    target: OwnershipTarget,
    path: PathBuf,
    bytes: u64,
    sha256: [u8; 32],
}

impl PublishedOwnershipArtifact {
    /// Returns the artifact target.
    #[must_use]
    pub const fn target(&self) -> OwnershipTarget {
        self.target
    }
    /// Returns the absolute published path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns the exact byte length.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    /// Returns the authenticated SHA-256 digest.
    #[must_use]
    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// One complete create-only candidate bundle.
#[derive(Clone, Debug)]
pub struct PublishedOwnershipBundle {
    command: CommandKind,
    path: PathBuf,
    manifest_path: PathBuf,
    artifacts: Vec<PublishedOwnershipArtifact>,
    results: Vec<OwnershipManifestResult>,
    diagnostics: Vec<Diagnostic>,
}

impl PublishedOwnershipBundle {
    /// Returns the authenticated provider warnings retained in manifest v3.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Returns build or run.
    #[must_use]
    pub const fn command(&self) -> CommandKind {
        self.command
    }
    /// Returns the absolute committed bundle directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns the strict manifest-v3 path.
    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }
    /// Returns artifacts in canonical target order.
    #[must_use]
    pub fn artifacts(&self) -> &[PublishedOwnershipArtifact] {
        &self.artifacts
    }
    /// Returns normalized run outcomes.
    #[must_use]
    pub fn results(&self) -> &[OwnershipManifestResult] {
        &self.results
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationPhase {
    JavaScript,
    WebAssembly,
    Native,
    Manifest,
    Commit,
}

type Checkpoint<'a> = &'a dyn Fn(PublicationPhase) -> Result<(), CommandFailure>;

pub(crate) fn publish_installed_build(
    prepared: &crate::ownership_pipeline::installed::PreparedInstalledOwnership<'_, '_>,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    if prepared.success().logical_export().is_some() {
        return Err(transaction_failure("installed run requires sealed execution evidence"));
    }
    prepared.revalidate()?;
    publish_with_checkpoint(prepared.success(), &|_| prepared.revalidate())
}

pub(crate) fn publish_installed_after_staging<Runner>(
    prepared: &crate::ownership_pipeline::installed::PreparedInstalledOwnership<'_, '_>,
    runner: Runner,
) -> Result<PublishedOwnershipBundle, CommandFailure>
where
    Runner: FnOnce(
        &Transaction,
        &ArtifactOutputRoot,
    ) -> Result<Vec<OwnershipManifestResult>, CommandFailure>,
{
    prepared.revalidate()?;
    publish_with_runner(prepared.success(), &|_| prepared.revalidate(), runner)
}

/// Publishes all selected artifacts and manifest v3 with one create-only directory commit.
///
/// The function either returns one complete bundle or removes its private transaction. Existing
/// destinations are never replaced.
///
/// # Errors
/// Returns stable preparation, publication, or cleanup diagnostics without a partial final path.
pub(crate) fn publish_data_ownership_build(
    success: &DataOwnershipCandidateSuccess,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    if success.logical_export().is_some() {
        return Err(transaction_failure(
            "candidate run publication requires sealed execution evidence",
        ));
    }
    publish_with_checkpoint(success, &|_| Ok(()))
}

fn publish_with_checkpoint(
    success: &DataOwnershipCandidateSuccess,
    checkpoint: Checkpoint<'_>,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    publish_with_runner(success, checkpoint, |_, _| Ok(Vec::new()))
}

pub(crate) fn publish_after_staging<Runner>(
    success: &DataOwnershipCandidateSuccess,
    runner: Runner,
) -> Result<PublishedOwnershipBundle, CommandFailure>
where
    Runner: FnOnce(
        &Transaction,
        &ArtifactOutputRoot,
    ) -> Result<Vec<OwnershipManifestResult>, CommandFailure>,
{
    publish_with_runner(success, &|_| Ok(()), runner)
}

pub(crate) fn publish_with_provider_checkpoint<Runner>(
    success: &DataOwnershipCandidateSuccess,
    validate: &dyn Fn() -> Result<(), CommandFailure>,
    runner: Runner,
) -> Result<PublishedOwnershipBundle, CommandFailure>
where
    Runner: FnOnce(
        &Transaction,
        &ArtifactOutputRoot,
    ) -> Result<Vec<OwnershipManifestResult>, CommandFailure>,
{
    publish_with_runner(success, &|_| validate(), runner)
}

fn publish_with_runner<Runner>(
    success: &DataOwnershipCandidateSuccess,
    checkpoint: Checkpoint<'_>,
    runner: Runner,
) -> Result<PublishedOwnershipBundle, CommandFailure>
where
    Runner: FnOnce(
        &Transaction,
        &ArtifactOutputRoot,
    ) -> Result<Vec<OwnershipManifestResult>, CommandFailure>,
{
    let command =
        if success.logical_export().is_some() { CommandKind::Run } else { CommandKind::Build };
    let suffix = match command {
        CommandKind::Build => "build",
        CommandKind::Run => "run",
    };
    let output = ArtifactOutputRoot::prepare_for_workspace(success.workspace_root())
        .map_err(preparation_failure)?;
    let bundle = output.path().join(format!("{}.{suffix}", success.artifact_stem()));
    let mut transaction = Transaction::create(&output)?;
    let operation: Result<PublishedOwnershipBundle, CommandFailure> = (|| {
        let artifacts = stage_artifacts(&transaction, success, &bundle, checkpoint)?;
        let results = runner(&transaction, &output)?;
        let manifest = render_ownership_manifest_v3(success, success.artifact_stem(), &results)
            .map_err(preparation_failure)?;
        decode_ownership_manifest_v3(&manifest).map_err(preparation_failure)?;
        transaction.write_manifest(OWNERSHIP_MANIFEST_NAME, &manifest)?;
        checkpoint(PublicationPhase::Manifest)?;
        checkpoint(PublicationPhase::Commit)?;
        transaction.commit(&output, &bundle)?;
        Ok(PublishedOwnershipBundle {
            command,
            manifest_path: bundle.join(OWNERSHIP_MANIFEST_NAME),
            path: bundle,
            artifacts,
            results,
            diagnostics: success.diagnostics().to_vec(),
        })
    })();
    match operation {
        Ok(bundle) => Ok(bundle),
        Err(mut failure) => {
            if let Err(cleanup) = transaction.cleanup(&output) {
                failure.kind = CommandFailureKind::Cleanup;
                failure.diagnostics.extend(cleanup.diagnostics);
            }
            Err(failure)
        }
    }
}

fn preparation_failure(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Preparation, diagnostics: vec![diagnostic] }
}

fn transaction_failure(message: impl Into<String>) -> CommandFailure {
    CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: vec![Diagnostic::error(
            "ZRYNA-C3201",
            None,
            message,
            "retry with one absent contained ownership bundle destination",
        )],
    }
}

#[cfg(test)]
mod tests;
