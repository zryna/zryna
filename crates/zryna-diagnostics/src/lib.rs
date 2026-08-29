//! Stable diagnostics shared across Zryna components.

#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Compilation must stop.
    Error,
    /// The operation may continue but should be reviewed.
    Warning,
}

/// A stable diagnostic produced by a Zryna component.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// Stable public diagnostic code.
    pub code: String,
    /// Severity of this diagnostic.
    pub severity: Severity,
    /// Optional workspace-relative path.
    pub path: Option<String>,
    /// Short problem statement.
    pub message: String,
    /// Concrete remediation guidance.
    pub guidance: String,
}

impl Diagnostic {
    /// Creates an error diagnostic.
    #[must_use]
    pub fn error(
        code: impl Into<String>,
        path: Option<String>,
        message: impl Into<String>,
        guidance: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            path,
            message: message.into(),
            guidance: guidance.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(path) = &self.path {
            write!(formatter, "{} [{}] {}: {}", self.code, path, self.message, self.guidance)
        } else {
            write!(formatter, "{}: {}: {}", self.code, self.message, self.guidance)
        }
    }
}
