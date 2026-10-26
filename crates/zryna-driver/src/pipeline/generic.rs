//! Ordinary generic artifacts share the existing audited atomic transaction.
use super::{CommandFailure, ManifestTarget, Transaction};
use crate::generic_manifest::{GenericArtifactRecord, GenericTarget};

impl Transaction {
    pub(crate) fn write_generic_artifact(
        &self,
        target: GenericTarget,
        stem: &str,
        bytes: &[u8],
    ) -> Result<GenericArtifactRecord, CommandFailure> {
        crate::javascript::validate_artifact_stem(stem).map_err(|diagnostic| CommandFailure {
            kind: super::CommandFailureKind::Preparation,
            diagnostics: vec![diagnostic],
        })?;
        let manifest_target = match target {
            GenericTarget::JavaScript => ManifestTarget::JavaScript,
            GenericTarget::WebAssembly => ManifestTarget::WebAssembly,
            GenericTarget::Native => ManifestTarget::Native,
        };
        let (kind, extension) = target.material();
        let artifact = self.write_artifact(manifest_target, kind, stem, extension, bytes)?;
        Ok(GenericArtifactRecord {
            target,
            kind: artifact.kind.to_owned(),
            path: artifact.path,
            bytes: artifact.bytes,
            sha256: artifact.sha256,
        })
    }
}
