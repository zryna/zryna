//! Provider-neutral source identifiers and spans.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable source-file identifier within one frontend snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct FileId(pub u32);

/// Half-open byte range in a source file.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    /// File containing the range.
    pub file: FileId,
    /// Inclusive UTF-8 byte offset.
    pub start: u32,
    /// Exclusive UTF-8 byte offset.
    pub end: u32,
}

impl Span {
    /// Creates a validated span.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when `start` is greater than `end`.
    pub fn new(file: FileId, start: u32, end: u32) -> Result<Self, zryna_diagnostics::Diagnostic> {
        if start > end {
            return Err(zryna_diagnostics::Diagnostic::error(
                "ZRYNA-S1001",
                None,
                "source span starts after it ends",
                "ensure every source range is half-open and ordered",
            ));
        }
        Ok(Self { file, start, end })
    }
}
