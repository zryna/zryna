//! Each selected backend receives the exact borrowed NEW owner.
use super::{
    CommandFailure, GenericNativeRecord, GenericTarget, OrdinaryArtifact, TargetSelection,
    VerifiedExecution,
};
use crate::generic_manifest::source::hex;
use zryna_diagnostics::Diagnostic;

pub(super) fn ordinary(
    program: &VerifiedExecution<'_>,
    targets: TargetSelection,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<(Vec<OrdinaryArtifact>, Option<GenericNativeRecord>), CommandFailure> {
    let mut artifacts = Vec::new();
    if targets.javascript() {
        checkpoint()?;
        let artifact =
            zryna_backend_javascript::generic_execution_v1::emit(program).map_err(failure)?;
        artifacts.push(OrdinaryArtifact {
            target: GenericTarget::JavaScript,
            bytes: artifact.source.into_bytes(),
        });
        checkpoint()?;
    }
    if targets.webassembly() {
        checkpoint()?;
        let artifact =
            zryna_backend_webassembly::generic_execution_v1::emit(program).map_err(failure)?;
        artifacts.push(OrdinaryArtifact {
            target: GenericTarget::WebAssembly,
            bytes: artifact.bytes().to_vec(),
        });
        checkpoint()?;
    }
    let native = if targets.native() {
        checkpoint()?;
        let mir = zryna_native_mir::generic_execution_v1::lower(program).map_err(failure)?;
        let target =
            zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
                .map_err(failure)?;
        let artifact = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
            .map_err(failure)?;
        let source = crate::generic_runtime_v1::RuntimeSource::issue(program, &mir, &artifact)
            .map_err(failure)?;
        let identity = source.identity();
        let record = GenericNativeRecord {
            target: identity.target().to_owned(),
            template_sha256: hex(identity.template_sha256()),
            header_sha256: hex(identity.header_sha256()),
            rendered_sha256: hex(identity.rendered_sha256()),
            ordinary_object_sha256: hex(identity.ordinary_object_sha256()),
            required_tls_bytes: identity.required_tls_bytes(),
            execution: None,
        };
        artifacts.push(OrdinaryArtifact {
            target: GenericTarget::Native,
            bytes: artifact.bytes().to_vec(),
        });
        checkpoint()?;
        Some(record)
    } else {
        None
    };
    Ok((artifacts, native))
}

fn failure(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: crate::CommandFailureKind::Preparation, diagnostics: vec![diagnostic] }
}
