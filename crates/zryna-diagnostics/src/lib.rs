//! Stable diagnostics resolved through the authoritative Zryna source map.

#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};
use zryna_source::{SourceError, SourceMap, Span};

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Compilation must stop.
    Error,
    /// The operation may continue but should be reviewed.
    Warning,
}

/// Mutually exclusive primary location for a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PrimaryLocation {
    /// Authoritative source range, resolved only through the matching source map.
    Source {
        /// Half-open UTF-8 byte span.
        span: Span,
    },
    /// Portable workspace path for architecture and repository errors.
    WorkspacePath {
        /// Workspace-relative path.
        path: String,
    },
    /// Locationless diagnostic.
    Global,
}

/// A stable diagnostic produced by a Zryna component.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// Stable public diagnostic code.
    pub code: String,
    /// Severity of this diagnostic.
    pub severity: Severity,
    primary: PrimaryLocation,
    /// Short problem statement.
    pub message: String,
    /// Concrete remediation guidance.
    pub guidance: String,
}

impl Diagnostic {
    /// Creates a path-scoped or global error diagnostic.
    #[must_use]
    pub fn error(
        code: impl Into<String>,
        path: Option<String>,
        message: impl Into<String>,
        guidance: impl Into<String>,
    ) -> Self {
        let primary =
            path.map_or(PrimaryLocation::Global, |path| PrimaryLocation::WorkspacePath { path });
        Self::new(code, Severity::Error, primary, message, guidance)
    }

    /// Creates an error whose primary location is an authoritative source span.
    #[must_use]
    pub fn error_at(
        code: impl Into<String>,
        span: Span,
        message: impl Into<String>,
        guidance: impl Into<String>,
    ) -> Self {
        Self::new(code, Severity::Error, PrimaryLocation::Source { span }, message, guidance)
    }

    /// Creates a path-scoped or global warning diagnostic.
    #[must_use]
    pub fn warning(
        code: impl Into<String>,
        path: Option<String>,
        message: impl Into<String>,
        guidance: impl Into<String>,
    ) -> Self {
        let primary =
            path.map_or(PrimaryLocation::Global, |path| PrimaryLocation::WorkspacePath { path });
        Self::new(code, Severity::Warning, primary, message, guidance)
    }

    /// Converts a source-boundary failure into a stable diagnostic.
    #[must_use]
    pub fn from_source_error(error: &SourceError) -> Self {
        Self::error(
            error.code(),
            error.path().map(str::to_owned),
            error.message(),
            error.guidance(),
        )
    }

    /// Returns the stable public diagnostic code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns this diagnostic's severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the primary location.
    #[must_use]
    pub const fn primary(&self) -> &PrimaryLocation {
        &self.primary
    }

    /// Returns a workspace path for path-scoped diagnostics.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        match &self.primary {
            PrimaryLocation::WorkspacePath { path } => Some(path),
            PrimaryLocation::Source { .. } | PrimaryLocation::Global => None,
        }
    }

    /// Returns the primary source span, when present.
    #[must_use]
    pub const fn primary_span(&self) -> Option<Span> {
        match self.primary {
            PrimaryLocation::Source { span } => Some(span),
            PrimaryLocation::WorkspacePath { .. } | PrimaryLocation::Global => None,
        }
    }

    /// Returns the short problem statement.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns concrete remediation guidance.
    #[must_use]
    pub fn guidance(&self) -> &str {
        &self.guidance
    }

    fn new(
        code: impl Into<String>,
        severity: Severity,
        primary: PrimaryLocation,
        message: impl Into<String>,
        guidance: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            primary,
            message: message.into(),
            guidance: guidance.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = escape_text(&self.code);
        let message = escape_text(&self.message);
        let guidance = escape_text(&self.guidance);
        match &self.primary {
            PrimaryLocation::Source { span } => write!(
                formatter,
                "{} [file#{}:{}..{}] {}: {}",
                code,
                span.file().index(),
                span.start(),
                span.end(),
                message,
                guidance
            ),
            PrimaryLocation::WorkspacePath { path } => {
                write!(formatter, "{} [{}] {}: {}", code, escape_text(path), message, guidance)
            }
            PrimaryLocation::Global => {
                write!(formatter, "{code}: {message}: {guidance}")
            }
        }
    }
}

/// Version of the stable structured diagnostic rendering schema.
pub const STRUCTURED_DIAGNOSTICS_VERSION: u32 = 1;

/// One fully resolved diagnostic in stable wire order.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedDiagnostic {
    /// Stable public code.
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Normalized source or workspace path, when present.
    pub path: Option<String>,
    /// Inclusive authoritative UTF-8 byte offset.
    pub byte_start: Option<u32>,
    /// Exclusive authoritative UTF-8 byte offset.
    pub byte_end: Option<u32>,
    /// One-based start line.
    pub line_start: Option<u32>,
    /// One-based Unicode-scalar start column.
    pub column_start: Option<u32>,
    /// One-based exclusive end line.
    pub line_end: Option<u32>,
    /// One-based exclusive Unicode-scalar end column.
    pub column_end: Option<u32>,
    /// Short problem statement.
    pub message: String,
    /// Concrete remediation guidance.
    pub guidance: String,
}

/// Versioned deterministic structured diagnostic report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredDiagnostics {
    /// Rendering schema version.
    pub schema_version: u32,
    /// Diagnostics in canonical location and content order.
    pub diagnostics: Vec<RenderedDiagnostic>,
}

/// Resolves and sorts diagnostics into the stable structured contract.
///
/// # Errors
///
/// Returns a source error if any source span does not belong to `sources`.
pub fn render_structured(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
) -> Result<StructuredDiagnostics, SourceError> {
    let mut rendered = diagnostics
        .iter()
        .map(|diagnostic| resolve_diagnostic(diagnostic, sources))
        .collect::<Result<Vec<_>, _>>()?;
    rendered.sort_by(|left, right| {
        (
            &left.path,
            left.byte_start,
            left.byte_end,
            left.severity,
            &left.code,
            &left.message,
            &left.guidance,
        )
            .cmp(&(
                &right.path,
                right.byte_start,
                right.byte_end,
                right.severity,
                &right.code,
                &right.message,
                &right.guidance,
            ))
    });
    Ok(StructuredDiagnostics {
        schema_version: STRUCTURED_DIAGNOSTICS_VERSION,
        diagnostics: rendered,
    })
}

/// Renders canonical compact JSON with stable field and diagnostic order.
///
/// # Errors
///
/// Returns an error if source resolution or serialization fails.
pub fn render_json(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
) -> Result<String, DiagnosticRenderError> {
    let report = render_structured(diagnostics, sources)?;
    Ok(serde_json::to_string(&report)?)
}

/// Failure while resolving or serializing deterministic diagnostics.
#[derive(Debug)]
pub enum DiagnosticRenderError {
    /// A source span did not belong to the supplied source map.
    Source(SourceError),
    /// The fixed structured output could not be serialized.
    Serialization(serde_json::Error),
}

impl fmt::Display for DiagnosticRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(formatter),
            Self::Serialization(error) => {
                write!(formatter, "diagnostic serialization failed: {error}")
            }
        }
    }
}

impl std::error::Error for DiagnosticRenderError {}

impl From<SourceError> for DiagnosticRenderError {
    fn from(error: SourceError) -> Self {
        Self::Source(error)
    }
}

impl From<serde_json::Error> for DiagnosticRenderError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Renders canonical LF-separated human-readable diagnostics.
///
/// # Errors
///
/// Returns a source error if any source span does not belong to `sources`.
pub fn render_text(diagnostics: &[Diagnostic], sources: &SourceMap) -> Result<String, SourceError> {
    let report = render_structured(diagnostics, sources)?;
    let mut lines = Vec::with_capacity(report.diagnostics.len());
    for diagnostic in report.diagnostics {
        let location = match (
            diagnostic.path.as_deref(),
            diagnostic.line_start,
            diagnostic.column_start,
            diagnostic.line_end,
            diagnostic.column_end,
            diagnostic.byte_start,
            diagnostic.byte_end,
        ) {
            (
                Some(path),
                Some(start_line),
                Some(start_column),
                Some(end_line),
                Some(end_column),
                Some(start),
                Some(end),
            ) => {
                let path = escape_text(path);
                format!(
                    "{path}:{start_line}:{start_column}-{end_line}:{end_column} [{start}..{end}]"
                )
            }
            (Some(path), None, None, None, None, None, None) => escape_text(path),
            _ => "<global>".to_owned(),
        };
        lines.push(format!(
            "{location}: {}[{}]: {}\n  guidance: {}",
            severity_name(diagnostic.severity),
            diagnostic.code,
            escape_text(&diagnostic.message),
            escape_text(&diagnostic.guidance)
        ));
    }
    Ok(lines.join("\n"))
}

fn resolve_diagnostic(
    diagnostic: &Diagnostic,
    sources: &SourceMap,
) -> Result<RenderedDiagnostic, SourceError> {
    let (path, byte_start, byte_end, line_start, column_start, line_end, column_end) =
        match &diagnostic.primary {
            PrimaryLocation::Source { span } => {
                let resolved = sources.resolve(*span)?;
                (
                    Some(resolved.source().path().as_str().to_owned()),
                    Some(resolved.start.byte_offset),
                    Some(resolved.end.byte_offset),
                    Some(resolved.start.line.saturating_add(1)),
                    Some(resolved.start.scalar_column.saturating_add(1)),
                    Some(resolved.end.line.saturating_add(1)),
                    Some(resolved.end.scalar_column.saturating_add(1)),
                )
            }
            PrimaryLocation::WorkspacePath { path } => {
                (Some(path.clone()), None, None, None, None, None, None)
            }
            PrimaryLocation::Global => (None, None, None, None, None, None, None),
        };
    Ok(RenderedDiagnostic {
        code: diagnostic.code.clone(),
        severity: diagnostic.severity,
        path,
        byte_start,
        byte_end,
        line_start,
        column_start,
        line_end,
        column_end,
        message: diagnostic.message.clone(),
        guidance: diagnostic.guidance.clone(),
    })
}

const fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\\' => escaped.push_str("\\\\"),
            character if character.is_control() => {
                use fmt::Write as _;
                let _ = write!(escaped, "\\u{{{:x}}}", u32::from(character));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_source::{NormalizedSourcePath, SourceFileInput};

    fn sources() -> SourceMap {
        SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "😀\nlet value = é;".to_owned(),
        }])
        .expect("fixture source map must be valid")
    }

    #[test]
    fn source_diagnostics_render_exact_text_and_json() {
        let sources = sources();
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture source must exist");
        let span = sources.span(file, 4, 4).expect("fixture span must be valid");
        let diagnostic = Diagnostic::error_at("ZRYNA-T1001", span, "bad\nvalue", "choose an i32");

        assert_eq!(
            render_text(std::slice::from_ref(&diagnostic), &sources)
                .expect("text rendering must succeed"),
            "src/main.zry:1:2-1:2 [4..4]: error[ZRYNA-T1001]: bad\\nvalue\n  guidance: choose an i32"
        );
        assert_eq!(
            render_json(&[diagnostic], &sources).expect("JSON rendering must succeed"),
            r#"{"schema_version":1,"diagnostics":[{"code":"ZRYNA-T1001","severity":"error","path":"src/main.zry","byte_start":4,"byte_end":4,"line_start":1,"column_start":2,"line_end":1,"column_end":2,"message":"bad\nvalue","guidance":"choose an i32"}]}"#
        );
    }

    #[test]
    fn rendering_order_is_deterministic_for_all_location_kinds() {
        let sources = sources();
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture source must exist");
        let source = Diagnostic::error_at(
            "ZRYNA-T1002",
            sources.span(file, 5, 8).expect("fixture span must be valid"),
            "source",
            "fix source",
        );
        let workspace = Diagnostic::error(
            "ZRYNA-A1001",
            Some("Cargo.toml".to_owned()),
            "workspace",
            "fix workspace",
        );
        let global = Diagnostic::error("ZRYNA-G1001", None, "global", "fix global");
        let forward = render_json(&[source.clone(), workspace.clone(), global.clone()], &sources)
            .expect("forward rendering must succeed");
        let reverse = render_json(&[global, workspace, source], &sources)
            .expect("reverse rendering must succeed");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn rendering_fails_closed_for_a_span_from_another_map() {
        let first = sources();
        let other = sources();
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = first.file_id(&path).expect("fixture source must exist");
        let diagnostic = Diagnostic::error_at(
            "ZRYNA-T1003",
            first.span(file, 0, 4).expect("fixture span must be valid"),
            "wrong map",
            "use the matching map",
        );
        let error = render_text(&[diagnostic], &other).expect_err("wrong map must fail");
        assert_eq!(error.code(), "ZRYNA-S1006");
    }

    #[test]
    fn display_escapes_control_characters_from_untrusted_diagnostics() {
        let diagnostic = Diagnostic::error(
            "ZRYNA-X1001\nspoof",
            Some("provider\npath".to_owned()),
            "message\rspoof",
            "guidance\tspoof",
        );

        assert_eq!(
            diagnostic.to_string(),
            "ZRYNA-X1001\\nspoof [provider\\npath] message\\rspoof: guidance\\tspoof"
        );
    }
}
