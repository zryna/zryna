//! Replaceable frontend-provider contracts.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zryna_diagnostics::Diagnostic;
use zryna_source::{FileId, Span};

/// Current wire contract understood by the compiler.
pub const FRONTEND_PROTOCOL_VERSION: u32 = 1;

/// Capabilities advertised by a frontend provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendCapabilities {
    /// Whether the provider resolves module specifiers.
    pub module_resolution: bool,
    /// Whether the provider supplies basic semantic diagnostics.
    pub semantic_diagnostics: bool,
}

/// Provider identity returned during handshake.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderInfo {
    /// Stable provider name.
    pub provider: String,
    /// Exact upstream provider version.
    pub provider_version: String,
    /// ZRYNA-owned protocol version.
    pub protocol_version: u32,
    /// Explicit provider capabilities.
    pub capabilities: FrontendCapabilities,
}

/// One source file supplied for analysis.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInput {
    /// Normalized workspace-relative path.
    pub path: String,
    /// UTF-8 source text.
    pub text: String,
}

/// Project analysis request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRequest {
    /// Request schema version.
    pub schema_version: u32,
    /// Complete bounded source set.
    pub files: Vec<SourceInput>,
}

/// Normalized type spelling retained for the Zryna semantic checker.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypeSyntax {
    /// No explicit annotation was present.
    Inferred,
    /// A named type reference such as `i32`.
    Named(String),
}

/// Normalized function declaration from a provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSyntax {
    /// Function name.
    pub name: String,
    /// Parameter names and annotations.
    pub parameters: Vec<(String, TypeSyntax)>,
    /// Declared return type.
    pub return_type: TypeSyntax,
    /// Source range for diagnostics.
    pub span: Span,
}

/// Provider-neutral source-unit snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceUnit {
    /// Snapshot-local identifier.
    pub id: FileId,
    /// Normalized path.
    pub path: String,
    /// Normalized declarations currently supported by the adapter.
    pub functions: Vec<FunctionSyntax>,
}

/// Provider-neutral immutable project snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSyntaxSnapshot {
    /// Snapshot schema version.
    pub schema_version: u32,
    /// Complete analyzed file set.
    pub files: Vec<SourceUnit>,
    /// Provider diagnostics; Zryna semantics remain authoritative.
    pub diagnostics: Vec<Diagnostic>,
}

/// Frontend-provider failure.
#[derive(Debug, Error)]
pub enum FrontendError {
    /// Provider speaks an incompatible protocol.
    #[error("frontend protocol mismatch: expected {expected}, received {actual}")]
    ProtocolMismatch {
        /// Required protocol.
        expected: u32,
        /// Received protocol.
        actual: u32,
    },
    /// Provider transport or execution failed.
    #[error("frontend provider failed: {0}")]
    Provider(String),
}

/// Replaceable source-analysis provider.
pub trait FrontendProvider: Send + Sync {
    /// Negotiates provider identity and capabilities.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider cannot start or complete its handshake.
    fn handshake(&self) -> Result<ProviderInfo, FrontendError>;

    /// Produces a provider-neutral immutable syntax snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when transport, provider execution, or snapshot production fails.
    fn analyze(&self, request: &AnalyzeRequest) -> Result<ProjectSyntaxSnapshot, FrontendError>;
}

/// Verifies that a provider can be used by this compiler.
///
/// # Errors
///
/// Returns a protocol mismatch when the provider does not speak the required Zryna contract.
pub fn verify_provider(info: &ProviderInfo) -> Result<(), FrontendError> {
    if info.protocol_version != FRONTEND_PROTOCOL_VERSION {
        return Err(FrontendError::ProtocolMismatch {
            expected: FRONTEND_PROTOCOL_VERSION,
            actual: info.protocol_version,
        });
    }
    Ok(())
}
