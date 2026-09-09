use std::{ffi::OsString, fmt, fs, path::Path};

use zryna_diagnostics::Diagnostic;
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, WorkerFrontend, WorkerLimits, WorkerSpec, syntax_v2,
};
use zryna_source::SourceMap;

use super::{DiagnosticRevision, DiagnosticSession, DiagnosticSessionError};
use crate::runtime::{NodeRuntimeCapability, node_compatible_path};

/// A pinned protocol-v2 compiler frontend retained by a tooling transport.
#[derive(Debug)]
pub struct ToolingCompiler {
    node: NodeRuntimeCapability,
    frontend: WorkerFrontend,
}

/// Failure to configure or use the bounded tooling compiler.
#[derive(Debug)]
pub enum ToolingCompilerError {
    /// Runtime, adapter, or worker configuration was unavailable or changed.
    Configuration(Diagnostic),
    /// Revision admission failed after analysis completed.
    Session(DiagnosticSessionError),
}

impl fmt::Display for ToolingCompilerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(diagnostic) => write!(formatter, "{diagnostic}"),
            Self::Session(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ToolingCompilerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Configuration(_) => None,
            Self::Session(error) => Some(error),
        }
    }
}

impl ToolingCompiler {
    /// Discovers the pinned runtime and fixed protocol-v2 adapter below one compiler workspace.
    ///
    /// # Errors
    ///
    /// Rejects non-absolute paths, links/reparse points, missing fixed adapter entries, the wrong
    /// runtime version, or an invalid worker specification.
    pub fn discover(root: &Path, node: &Path) -> Result<Self, ToolingCompilerError> {
        if !root.is_absolute() || !node.is_absolute() {
            return Err(configuration_error(
                "tooling compiler root and Node.js runtime must be absolute paths",
                "pass the absolute compiler workspace and Node.js 22.22.1 executable",
            ));
        }
        validate_real_directory(root, "tooling compiler root")?;
        let adapter = root.join("adapters/typescript-6");
        validate_real_directory(&adapter, "TypeScript adapter directory")?;
        let worker = adapter.join("src/worker.mjs");
        validate_real_file(&worker, "TypeScript frontend worker")?;
        let node = NodeRuntimeCapability::discover(node, root)
            .map_err(ToolingCompilerError::Configuration)?;
        let expected = ProviderExpectation::new(
            "typescript-6",
            "6.0.3",
            syntax_v2::PROTOCOL_VERSION,
            FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
        )
        .map_err(|_| {
            configuration_error(
                "the fixed tooling frontend expectation is invalid",
                "restore the registered protocol-v2 adapter contract",
            )
        })?;
        let spec = WorkerSpec::new(
            node.executable().map_err(ToolingCompilerError::Configuration)?,
            vec![OsString::from(node_compatible_path(&worker))],
            node_compatible_path(&adapter),
            expected,
            WorkerLimits::default(),
        )
        .map_err(|_| {
            configuration_error(
                "the fixed tooling frontend process could not be configured",
                "restore the registered adapter and pinned runtime paths",
            )
        })?;
        Ok(Self { node, frontend: WorkerFrontend::new(spec) })
    }

    /// Analyzes and admits one exact in-memory source revision.
    ///
    /// Frontend failures are retained as their authoritative diagnostic report. Semantic facts are
    /// retained only after the existing scalar checker succeeds.
    ///
    /// # Errors
    ///
    /// Rejects runtime replacement or a failed revision admission.
    pub fn admit(
        &self,
        session: &mut DiagnosticSession,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, ToolingCompilerError> {
        self.node.revalidate().map_err(ToolingCompilerError::Configuration)?;
        let revision = match crate::analyze_sources(&self.frontend, &sources) {
            Ok(syntax) => session.admit_analysis(sources, &syntax),
            Err(error) => session.admit_diagnostics(sources, error.diagnostics()),
        }
        .map_err(ToolingCompilerError::Session)?;
        self.node.revalidate().map_err(ToolingCompilerError::Configuration)?;
        Ok(revision)
    }
}

fn validate_real_directory(path: &Path, label: &str) -> Result<(), ToolingCompilerError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        configuration_error(
            format!("{label} is unavailable"),
            "restore the registered compiler workspace without links",
        )
    })?;
    if metadata.is_dir() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(configuration_error(
            format!("{label} is not a real directory"),
            "restore the registered compiler workspace without links",
        ))
    }
}

fn validate_real_file(path: &Path, label: &str) -> Result<(), ToolingCompilerError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        configuration_error(
            format!("{label} is unavailable"),
            "restore the fixed registered adapter entrypoint",
        )
    })?;
    if metadata.is_file() && !metadata_is_link_or_reparse(&metadata) {
        Ok(())
    } else {
        Err(configuration_error(
            format!("{label} is not a real regular file"),
            "restore the fixed registered adapter entrypoint",
        ))
    }
}

fn configuration_error(message: impl Into<String>, guidance: &'static str) -> ToolingCompilerError {
    ToolingCompilerError::Configuration(Diagnostic::error("ZRYNA-D3001", None, message, guidance))
}

#[cfg(unix)]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
