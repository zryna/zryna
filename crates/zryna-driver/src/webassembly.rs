//! Core-WebAssembly-only source compilation and create-only artifact publication.

use std::{error::Error, fmt, path::Path};

use zryna_backend_webassembly::ValidatedWebAssemblyArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_source::SourceMap;

use crate::{
    SourceToIrError, compile_to_verified_ir,
    javascript::{ArtifactOutputRoot, MAX_ARTIFACT_STEM_BYTES, publish_complete_artifact},
};

/// File extension used for core WebAssembly modules.
pub const WEBASSEMBLY_ARTIFACT_EXTENSION: &str = "wasm";
/// Validated capability for the workspace's declared `.zryna/out` output root.
pub type WebAssemblyOutputRoot = ArtifactOutputRoot;
/// Maximum portable artifact stem bytes accepted by the WebAssembly publisher.
pub const MAX_WEBASSEMBLY_ARTIFACT_STEM_BYTES: usize = MAX_ARTIFACT_STEM_BYTES;

/// One validated WebAssembly module atomically published at a new destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedWebAssemblyArtifact {
    path: std::path::PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl PublishedWebAssemblyArtifact {
    /// Returns the exact published module path.
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

/// Successful source-to-WebAssembly compilation.
#[derive(Clone, Debug)]
pub struct WebAssemblyBuildSuccess {
    artifact: PublishedWebAssemblyArtifact,
    diagnostics: Vec<Diagnostic>,
}

impl WebAssemblyBuildSuccess {
    /// Returns the atomically published validated module.
    #[must_use]
    pub const fn artifact(&self) -> &PublishedWebAssemblyArtifact {
        &self.artifact
    }

    /// Returns deterministic non-fatal frontend and publication diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Failure before a new WebAssembly artifact can be reported.
#[derive(Debug)]
pub enum WebAssemblyBuildError {
    /// Source analysis, semantics, or IR verification failed.
    Source(SourceToIrError),
    /// The WebAssembly backend rejected or could not validate a verified program.
    Backend(Diagnostic),
    /// The complete module could not be published atomically.
    Publication(Diagnostic),
}

impl WebAssemblyBuildError {
    /// Returns deterministic compiler diagnostics carried by this failure.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Source(error) => error.diagnostics(),
            Self::Backend(diagnostic) | Self::Publication(diagnostic) => {
                std::slice::from_ref(diagnostic)
            }
        }
    }
}

impl fmt::Display for WebAssemblyBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Backend(diagnostic) => {
                write!(formatter, "WebAssembly emission failed: {diagnostic}")
            }
            Self::Publication(diagnostic) => {
                write!(formatter, "WebAssembly publication failed: {diagnostic}")
            }
        }
    }
}

impl Error for WebAssemblyBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Backend(_) | Self::Publication(_) => None,
        }
    }
}

/// Compiles real source directly through verified IR to validated core WebAssembly.
///
/// Production success is encode, pinned validation, strict profile audit, and create-only
/// publication. Runtime instantiation belongs to conformance tests and is not a per-build phase.
///
/// # Errors
///
/// Returns a phase-specific failure and leaves no new destination on failure.
pub fn compile_webassembly<Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
    output_root: &WebAssemblyOutputRoot,
    artifact_stem: &str,
) -> Result<WebAssemblyBuildSuccess, WebAssemblyBuildError> {
    let compiled =
        compile_to_verified_ir(frontend, sources).map_err(WebAssemblyBuildError::Source)?;
    let artifact = zryna_backend_webassembly::emit(compiled.program())
        .map_err(WebAssemblyBuildError::Backend)?;
    let published = publish_webassembly(&artifact, output_root, artifact_stem)
        .map_err(WebAssemblyBuildError::Publication)?;
    let mut diagnostics = compiled.diagnostics().to_vec();
    diagnostics.extend_from_slice(published.diagnostics());
    Ok(WebAssemblyBuildSuccess { artifact: published, diagnostics })
}

/// Atomically publishes one sealed, validated WebAssembly artifact at an absent destination.
///
/// # Errors
///
/// Returns a stable diagnostic without creating or modifying the destination on failure.
pub fn publish_webassembly(
    artifact: &ValidatedWebAssemblyArtifact,
    output_root: &WebAssemblyOutputRoot,
    artifact_stem: &str,
) -> Result<PublishedWebAssemblyArtifact, Diagnostic> {
    let published = publish_complete_artifact(
        artifact.bytes(),
        output_root,
        artifact_stem,
        WEBASSEMBLY_ARTIFACT_EXTENSION,
        "WebAssembly",
    )?;
    Ok(PublishedWebAssemblyArtifact { path: published.path, diagnostics: published.diagnostics })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use zryna_ir::{Expr, ExprId, ExprKind, Function, Program, Type, verify};
    use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

    fn artifact() -> zryna_backend_webassembly::ValidatedWebAssemblyArtifact {
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/add.zry".to_owned(),
            text: "x".to_owned(),
        }])
        .expect("fixture source map");
        let path = NormalizedSourcePath::new("src/add.zry").expect("fixture path");
        let file = sources.file_id(&path).expect("fixture file");
        let span = sources.span(file, 0, 1).expect("fixture span");
        let program = Program {
            functions: vec![Function {
                name: "add".to_owned(),
                parameters: vec![Type::I32, Type::I32],
                return_type: Type::I32,
                expressions: vec![
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(0) },
                    Expr { ty: Type::I32, span, kind: ExprKind::Parameter(1) },
                    Expr {
                        ty: Type::I32,
                        span,
                        kind: ExprKind::I32Add { lhs: ExprId(0), rhs: ExprId(1) },
                    },
                ],
                body: ExprId(2),
            }],
        };
        let verified = verify(program, &sources).expect("fixture IR");
        zryna_backend_webassembly::emit(&verified).expect("fixture module")
    }

    #[test]
    fn publication_is_create_only_and_coexists_with_javascript() {
        let workspace =
            std::env::temp_dir().join(format!("zryna-driver-webassembly-{}", std::process::id()));
        let output_path = workspace.join(".zryna/out");
        fs::create_dir_all(&output_path).expect("fixture output");
        let output =
            super::WebAssemblyOutputRoot::for_workspace(&workspace).expect("fixture capability");
        fs::write(output_path.join("main.mjs"), b"export {};\n").expect("JavaScript fixture");

        let artifact = artifact();
        let published = super::publish_webassembly(&artifact, &output, "main")
            .expect("fresh WebAssembly destination");
        assert_eq!(published.path(), output_path.join("main.wasm"));
        assert_eq!(fs::read(published.path()).expect("published bytes"), artifact.bytes());
        fs::write(published.path(), b"sentinel").expect("distinct existing destination");
        let diagnostic = super::publish_webassembly(&artifact, &output, "main")
            .expect_err("existing WebAssembly destination must be preserved");
        assert_eq!(diagnostic.code(), "ZRYNA-D2007");
        assert_eq!(fs::read(published.path()).expect("preserved bytes"), b"sentinel");
        assert_eq!(fs::read_dir(&output_path).expect("output listing").count(), 2);

        fs::remove_dir_all(workspace).expect("fixture cleanup");
    }
}
