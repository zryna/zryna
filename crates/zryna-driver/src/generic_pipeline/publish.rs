//! Terminal publication keeps the source checkpoint immediately before atomic commit.
use super::{PreparedGenericBuild, rejected};
use crate::generic_manifest::{GenericCommand, GenericInvocationRecord, GenericManifestWire};
use crate::pipeline::Transaction;
use crate::{ArtifactOutputRoot, CommandFailure, generic_manifest as manifest};
use std::path::{Path, PathBuf};
use zryna_abi::ScalarOutcome;
use zryna_diagnostics::Diagnostic;

/// One completed target observation in canonical target order.
#[derive(Clone, Debug, serde::Serialize)]
pub struct GenericTargetResult {
    target: &'static str,
    outcome: ScalarOutcome,
}
impl GenericTargetResult {
    /// Canonical selected target label.
    #[must_use]
    pub const fn target(&self) -> &'static str {
        self.target
    }
    /// Completed scalar return or controlled source trap.
    #[must_use]
    pub const fn outcome(&self) -> ScalarOutcome {
        self.outcome
    }
}

/// An owned committed result; it retains no compiler or source authority.
#[derive(Clone, Debug)]
pub struct PublishedGenericBundle {
    bundle: PathBuf,
    manifest: PathBuf,
    manifest_bytes: Vec<u8>,
    command: &'static str,
    results: Vec<GenericTargetResult>,
    diagnostics: Vec<Diagnostic>,
}
impl PublishedGenericBundle {
    /// Directory atomically committed by this command.
    #[must_use]
    pub fn bundle_path(&self) -> &Path {
        &self.bundle
    }
    /// The separate versioned generic manifest.
    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest
    }
    /// Exact committed plain metadata, with no execution authority.
    #[must_use]
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }
    /// The completed command, either build or run.
    #[must_use]
    pub const fn command(&self) -> &'static str {
        self.command
    }
    /// Canonical completed observations; empty for a build.
    #[must_use]
    pub fn results(&self) -> &[GenericTargetResult] {
        &self.results
    }
    /// Retained authenticated source advisories.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

pub(crate) fn publish(
    prepared: PreparedGenericBuild,
    output: &ArtifactOutputRoot,
    stem: &str,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<PublishedGenericBundle, CommandFailure> {
    publish_document(prepared, output, stem, None, checkpoint)
}

pub(crate) fn publish_run(
    mut prepared: PreparedGenericBuild,
    completed: crate::generic_run::CompletedGenericRun,
    output: &ArtifactOutputRoot,
    stem: &str,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<PublishedGenericBundle, CommandFailure> {
    let (record, native) = completed.into_records(&prepared.receipt)?;
    match (&mut prepared.native, native) {
        (Some(material), Some(execution)) => material.execution = Some(execution),
        (None, None) => {}
        _ => {
            return Err(rejected("completed native run does not match its ordinary artifacts"));
        }
    }
    publish_document(prepared, output, stem, Some(record), checkpoint)
}

// Plain wire data enters only from build(None) or the source-bound opaque completion above.
fn publish_document(
    prepared: PreparedGenericBuild,
    output: &ArtifactOutputRoot,
    stem: &str,
    invocation: Option<GenericInvocationRecord>,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<PublishedGenericBundle, CommandFailure> {
    crate::javascript::validate_artifact_stem(stem)
        .map_err(|_| rejected("generic output stem is invalid"))?;
    checkpoint()?;
    let command = if invocation.is_some() { "run" } else { "build" };
    let bundle = output.path().join(format!("{stem}.{command}"));
    crate::pipeline::ensure_absent(&bundle)?;
    let mut transaction = Transaction::create(output)?;
    let result = (|| {
        let mut artifacts = Vec::new();
        for artifact in prepared.artifacts {
            checkpoint()?;
            artifacts.push(transaction.write_generic_artifact(
                artifact.target,
                stem,
                &artifact.bytes,
            )?);
        }
        let document = GenericManifestWire {
            format: manifest::FORMAT.to_owned(),
            profile: prepared.receipt.profile().to_owned(),
            syntax_protocol: prepared.receipt.syntax_protocol(),
            command: if invocation.is_some() { GenericCommand::Run } else { GenericCommand::Build },
            artifact_stem: stem.to_owned(),
            source: manifest::source::receipt(&prepared.receipt),
            exports: manifest::source::exports(&prepared.receipt),
            artifacts,
            native: prepared.native,
            invocation,
        };
        let bytes = manifest::encode(&document).map_err(|diagnostic| CommandFailure {
            kind: crate::CommandFailureKind::Preparation,
            diagnostics: vec![diagnostic],
        })?;
        checkpoint()?;
        transaction.write_manifest(manifest::MANIFEST_NAME, &bytes)?;
        // Prepare all owned result data before commit. No later source callback may fail.
        let published = PublishedGenericBundle {
            manifest: bundle.join(manifest::MANIFEST_NAME),
            bundle,
            manifest_bytes: bytes,
            command,
            results: document.invocation.as_ref().map_or_else(Vec::new, |record| {
                record
                    .outcomes
                    .iter()
                    .map(|row| GenericTargetResult {
                        target: row.target.as_str(),
                        outcome: row.outcome,
                    })
                    .collect()
            }),
            diagnostics: prepared.diagnostics,
        };
        transaction.commit_guarded(output, &published.bundle, checkpoint)?;
        Ok(published)
    })();
    match result {
        Ok(published) => Ok(published),
        Err(failure) => match transaction.cleanup(output) {
            Ok(()) => Err(failure),
            Err(cleanup) => Err(cleanup),
        },
    }
}
