//! Linux x86-64 native-object compilation and create-only publication.

use std::{error::Error, fmt, path::Path};

use zryna_backend_native::{
    LinuxX8664ObjectTarget, ValidatedNativeObjectArtifact, select_object_target,
};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_source::SourceMap;

use crate::{
    SourceToIrError, compile_to_verified_ir,
    javascript::{ArtifactOutputRoot, MAX_ARTIFACT_STEM_BYTES, publish_complete_artifact},
};

/// File extension used for Linux x86-64 ELF relocatable objects.
pub const NATIVE_OBJECT_ARTIFACT_EXTENSION: &str = "o";
/// Validated capability for the workspace's declared `.zryna/out` output root.
pub type NativeObjectOutputRoot = ArtifactOutputRoot;
/// Maximum portable artifact stem bytes accepted by the native object publisher.
pub const MAX_NATIVE_OBJECT_ARTIFACT_STEM_BYTES: usize = MAX_ARTIFACT_STEM_BYTES;

/// One audited native object atomically published at a new destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedNativeObjectArtifact {
    path: std::path::PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl PublishedNativeObjectArtifact {
    /// Returns the exact published object path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns non-fatal publication diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Successful source-to-native-object compilation.
#[derive(Clone, Debug)]
pub struct NativeObjectBuildSuccess {
    artifact: PublishedNativeObjectArtifact,
    diagnostics: Vec<Diagnostic>,
}

impl NativeObjectBuildSuccess {
    /// Returns the atomically published audited object.
    #[must_use]
    pub const fn artifact(&self) -> &PublishedNativeObjectArtifact {
        &self.artifact
    }

    /// Returns deterministic non-fatal frontend and publication diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Failure before a new native object can be reported.
#[derive(Debug)]
pub enum NativeObjectBuildError {
    /// Source analysis, semantics, or IR verification failed.
    Source(SourceToIrError),
    /// Native MIR lowering rejected the verified program.
    Mir(Vec<Diagnostic>),
    /// Target selection or native object emission failed.
    Backend(Diagnostic),
    /// The complete object could not be published atomically.
    Publication(Diagnostic),
}

impl NativeObjectBuildError {
    /// Returns deterministic compiler diagnostics carried by this failure.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Source(error) => error.diagnostics(),
            Self::Mir(diagnostics) => diagnostics,
            Self::Backend(diagnostic) | Self::Publication(diagnostic) => {
                std::slice::from_ref(diagnostic)
            }
        }
    }
}

impl fmt::Display for NativeObjectBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Mir(diagnostics) => {
                write!(
                    formatter,
                    "native MIR lowering failed with {} diagnostic(s)",
                    diagnostics.len()
                )
            }
            Self::Backend(diagnostic) => {
                write!(formatter, "native object emission failed: {diagnostic}")
            }
            Self::Publication(diagnostic) => {
                write!(formatter, "native object publication failed: {diagnostic}")
            }
        }
    }
}

impl Error for NativeObjectBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Mir(_) | Self::Backend(_) | Self::Publication(_) => None,
        }
    }
}

/// Compiles real source through verified IR and native MIR to one audited ELF object.
///
/// Target selection happens before code generation and publication is create-only. No product
/// linker or generated executable is invoked.
///
/// # Errors
///
/// Returns a phase-specific failure and leaves no new destination on failure.
pub fn compile_native_object<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
    output_root: &NativeObjectOutputRoot,
    artifact_stem: &str,
    requested_target: &str,
) -> Result<NativeObjectBuildSuccess, NativeObjectBuildError> {
    let target = select_object_target(requested_target).map_err(NativeObjectBuildError::Backend)?;
    let compiled =
        compile_to_verified_ir(frontend, sources).map_err(NativeObjectBuildError::Source)?;
    let mir = zryna_native_mir::lower(compiled.program()).map_err(NativeObjectBuildError::Mir)?;
    let artifact =
        zryna_backend_native::emit_object(&mir, target).map_err(NativeObjectBuildError::Backend)?;
    let published = publish_native_object(&artifact, output_root, artifact_stem)
        .map_err(NativeObjectBuildError::Publication)?;
    let mut diagnostics = compiled.diagnostics().to_vec();
    diagnostics.extend_from_slice(published.diagnostics());
    Ok(NativeObjectBuildSuccess { artifact: published, diagnostics })
}

/// Atomically publishes one sealed, audited native object at an absent destination.
///
/// # Errors
///
/// Returns a stable diagnostic without creating or modifying the destination on failure.
pub fn publish_native_object(
    artifact: &ValidatedNativeObjectArtifact,
    output_root: &NativeObjectOutputRoot,
    artifact_stem: &str,
) -> Result<PublishedNativeObjectArtifact, Diagnostic> {
    let published = publish_complete_artifact(
        artifact.bytes(),
        output_root,
        artifact_stem,
        NATIVE_OBJECT_ARTIFACT_EXTENSION,
        "native object",
    )?;
    Ok(PublishedNativeObjectArtifact { path: published.path, diagnostics: published.diagnostics })
}

/// Selects the exact native target for callers that need to validate configuration first.
///
/// # Errors
///
/// Returns the backend's stable unsupported-target diagnostic.
pub fn select_native_object_target(
    requested_target: &str,
) -> Result<LinuxX8664ObjectTarget, Diagnostic> {
    select_object_target(requested_target)
}
