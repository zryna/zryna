//! Opt-in bounded diagnostic transport. Text and JSON v1 remain unchanged.
//!
//! See `spec/diagnostics/STRUCTURED_DIAGNOSTICS_V2.md` for the wire contract.

use std::fmt;

use zryna_source::{NormalizedSourcePath, SourceMap};

use crate::{Diagnostic, PrimaryLocation};

mod wire;
use wire::{Location, Record, Report};

#[cfg(test)]
mod tests;

/// Maximum UTF-8 bytes in one complete JSON document, including whitespace.
pub const MAX_DOCUMENT_BYTES: usize = 65_536;
/// Maximum records in a complete report.
pub const MAX_DIAGNOSTICS: usize = 256;
/// Maximum UTF-8 bytes in each message or guidance string.
pub const MAX_TEXT_BYTES: usize = 4_096;
/// Maximum UTF-8 bytes in each source or workspace display path.
pub const MAX_PATH_BYTES: usize = 1_024;
/// Reserved transport exhaustion code; never a language diagnostic.
pub const EXHAUSTION_CODE: &str = "ZRYNA-D2001";

/// Stable rejection categories for untrusted transport documents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    /// The document or a decoded collection/string exceeds its budget.
    Limit,
    /// Invalid JSON, duplicate/unknown/missing fields, or wrong field types.
    Shape,
    /// The explicitly selected protocol version is not 2.
    Version,
    /// Malformed code or a forged/nonterminal exhaustion record.
    Record,
    /// Source path or span does not resolve in the supplied snapshot.
    Source,
    /// Records are not in canonical location/content order.
    Order,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Limit => "diagnostic protocol limit exceeded",
            Self::Shape => "invalid diagnostic protocol shape",
            Self::Version => "unsupported diagnostic protocol version",
            Self::Record => "invalid diagnostic protocol record",
            Self::Source => "diagnostic source does not match the supplied snapshot",
            Self::Order => "diagnostics are not in canonical order",
        })
    }
}

impl std::error::Error for ProtocolError {}

/// Renders a complete v2 document or one fixed terminal exhaustion record.
///
/// Count, decoded string, then encoded document limits are inclusive. Exhaustion
/// discards the entire report, so input permutations select identical output.
/// Callers retain ownership of the input diagnostics and source snapshot.
///
/// # Errors
///
/// Returns `Record` for invalid/reserved codes or `Source` for a mismatched span.
pub fn render_json(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
) -> Result<String, ProtocolError> {
    if diagnostics.len() > MAX_DIAGNOSTICS
        || diagnostics.iter().any(|diagnostic| {
            diagnostic.message().len() > MAX_TEXT_BYTES
                || diagnostic.guidance().len() > MAX_TEXT_BYTES
                || diagnostic.path().is_some_and(|path| path.len() > MAX_PATH_BYTES)
        })
    {
        return encode(&Report::exhausted());
    }
    let mut records = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        if !valid_code(diagnostic.code()) || diagnostic.code() == EXHAUSTION_CODE {
            return Err(ProtocolError::Record);
        }
        let location = match diagnostic.primary() {
            PrimaryLocation::Global => Location::Global {},
            PrimaryLocation::WorkspacePath { path } => {
                Location::WorkspacePath { path: path.clone() }
            }
            PrimaryLocation::Source { span } => {
                let resolved = sources.resolve(*span).map_err(|_| ProtocolError::Source)?;
                Location::Source {
                    path: resolved.source().path().as_str().to_owned(),
                    byte_start: resolved.start.byte_offset,
                    byte_end: resolved.end.byte_offset,
                }
            }
        };
        records.push(Record {
            code: diagnostic.code().to_owned(),
            severity: diagnostic.severity(),
            location,
            message: diagnostic.message().to_owned(),
            guidance: diagnostic.guidance().to_owned(),
        });
    }
    records.sort_by(|left, right| left.key().cmp(&right.key()));
    let encoded = encode(&Report { schema_version: 2, diagnostics: records })?;
    if encoded.len() > MAX_DOCUMENT_BYTES { encode(&Report::exhausted()) } else { Ok(encoded) }
}

/// Validates an untrusted v2 document against the caller's exact source snapshot.
///
/// This grants no compiler authority and constructs no `Diagnostic` or `Span`
/// for the caller. The caller must bind the response to its compilation request;
/// matching paths/ranges alone cannot authenticate stale or malicious messages.
///
/// # Errors
///
/// Rejects oversized bytes before parsing; then rejects shape, version, decoded
/// limits, record contents, source mismatch, and ordering, in that order.
pub fn validate_json(bytes: &[u8], sources: &SourceMap) -> Result<(), ProtocolError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(ProtocolError::Limit);
    }
    let report: Report = serde_json::from_slice(bytes).map_err(|_| ProtocolError::Shape)?;
    if report.schema_version != 2 {
        return Err(ProtocolError::Version);
    }
    if report.diagnostics.len() > MAX_DIAGNOSTICS
        || report.diagnostics.iter().any(Record::exceeds_limits)
    {
        return Err(ProtocolError::Limit);
    }
    if report.diagnostics.iter().any(|record| !valid_code(&record.code)) {
        return Err(ProtocolError::Record);
    }
    if report.diagnostics.iter().any(|record| record.code == EXHAUSTION_CODE) {
        return if report == Report::exhausted() { Ok(()) } else { Err(ProtocolError::Record) };
    }
    for record in &report.diagnostics {
        if let Location::Source { path, byte_start, byte_end } = &record.location {
            let path =
                NormalizedSourcePath::new(path.clone()).map_err(|_| ProtocolError::Source)?;
            let file = sources.file_id(&path).ok_or(ProtocolError::Source)?;
            if sources.source(file).is_none_or(|source| source.path() != &path) {
                return Err(ProtocolError::Source);
            }
            sources.span(file, *byte_start, *byte_end).map_err(|_| ProtocolError::Source)?;
        }
    }
    if report.diagnostics.windows(2).any(|pair| pair[0].key() > pair[1].key()) {
        return Err(ProtocolError::Order);
    }
    Ok(())
}

fn valid_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 11
        && bytes.starts_with(b"ZRYNA-")
        && bytes[6].is_ascii_uppercase()
        && bytes[7..].iter().all(u8::is_ascii_digit)
}

fn encode(report: &Report) -> Result<String, ProtocolError> {
    serde_json::to_string(report).map_err(|_| ProtocolError::Shape)
}
