use std::{ffi::OsString, fmt, path::Path};

use zryna_diagnostics::Diagnostic;
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, WorkerFrontend, WorkerLimits, WorkerSpec, syntax_v2,
};
use zryna_source::SourceMap;

use super::tooling_execution::ToolingExecutionClosure;
use super::{DiagnosticRevision, DiagnosticSession, DiagnosticSessionError};
use crate::runtime::{NodeRuntimeCapability, node_compatible_path};

/// A pinned protocol-v2 compiler frontend retained by a tooling transport.
#[derive(Debug)]
pub struct ToolingCompiler {
    node: NodeRuntimeCapability,
    frontend: WorkerFrontend,
    execution: ToolingExecutionClosure,
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
        let execution =
            ToolingExecutionClosure::capture(root).map_err(ToolingCompilerError::Configuration)?;
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
            vec![OsString::from(node_compatible_path(execution.worker()))],
            node_compatible_path(execution.working_directory()),
            expected,
            WorkerLimits::default(),
        )
        .map_err(|_| {
            configuration_error(
                "the fixed tooling frontend process could not be configured",
                "restore the registered adapter and pinned runtime paths",
            )
        })?;
        Ok(Self { node, frontend: WorkerFrontend::new(spec), execution })
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
        self.execution.revalidate().map_err(ToolingCompilerError::Configuration)?;
        let analysis = crate::analyze_sources(&self.frontend, &sources);
        self.execution.revalidate().map_err(ToolingCompilerError::Configuration)?;
        self.node.revalidate().map_err(ToolingCompilerError::Configuration)?;
        match analysis {
            Ok(syntax) => session.admit_analysis(sources, &syntax),
            Err(error) => session.admit_diagnostics(sources, error.diagnostics()),
        }
        .map_err(ToolingCompilerError::Session)
    }
}

fn configuration_error(message: impl Into<String>, guidance: &'static str) -> ToolingCompilerError {
    ToolingCompilerError::Configuration(Diagnostic::error("ZRYNA-D3001", None, message, guidance))
}
