//! Authoritative, provider-neutral source files, identifiers, and spans.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Maximum number of source files in one immutable source map.
pub const MAX_SOURCE_FILES: usize = 4_096;
/// Maximum UTF-8 bytes in one source file.
pub const MAX_SOURCE_FILE_BYTES: usize = 2 * 1_024 * 1_024;
/// Maximum aggregate UTF-8 bytes in one source map.
pub const MAX_SOURCE_BYTES: usize = 64 * 1_024 * 1_024;
/// Maximum UTF-8 bytes in a portable source path.
pub const MAX_SOURCE_PATH_BYTES: usize = 1_024;
/// Maximum components in a portable source path.
pub const MAX_SOURCE_PATH_COMPONENTS: usize = 32;

static NEXT_SOURCE_MAP_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one immutable [`SourceMap`] authority.
///
/// Clones of a source map retain this identity. Independently built maps never compare equal,
/// including empty maps whose file sets otherwise provide no identity witness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMapIdentity(u64);

/// Stable source-file identifier within one [`SourceMap`].
///
/// Identifiers are assigned by normalized path order and are not globally persistent.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileId {
    source_map_id: u64,
    index: u32,
}

impl FileId {
    /// Returns the dense zero-based identifier value.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }
}

impl Serialize for FileId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.index)
    }
}

/// A validated portable workspace-relative source path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NormalizedSourcePath(String);

impl NormalizedSourcePath {
    /// Validates a source path without consulting host filesystem semantics.
    ///
    /// # Errors
    ///
    /// Returns a stable source error for non-portable or oversized paths.
    pub fn new(path: impl Into<String>) -> Result<Self, SourceError> {
        let path = path.into();
        validate_source_path(&path)?;
        Ok(Self(path))
    }

    /// Returns the normalized `/`-separated path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the frozen ASCII case-insensitive identity used for portable collision checks.
    ///
    /// The returned value is not a host path and must never be used for filesystem lookup.
    #[must_use]
    pub fn portable_identity(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}

/// Resolves one explicit relative lowercase `.zry` import without consulting a host filesystem.
///
/// This is the shared portable grammar used by module discovery and semantic revalidation. It
/// deliberately performs only lexical path resolution; callers retain authority for filesystem
/// access, graph closure, and source-map membership.
///
/// # Errors
///
/// Rejects non-relative, non-ASCII, host-specific, escaping, implicit-extension, URL-like, query,
/// fragment, empty-component, and otherwise non-portable specifiers.
pub fn resolve_explicit_zry_import(
    importer: &NormalizedSourcePath,
    specifier: &str,
) -> Result<NormalizedSourcePath, InvalidModuleSpecifier> {
    if !(specifier.starts_with("./") || specifier.starts_with("../"))
        || !has_exact_zry_extension(specifier)
        || !specifier.is_ascii()
        || specifier.contains(['\\', '?', '#', '\0'])
        || specifier.contains("://")
    {
        return Err(InvalidModuleSpecifier);
    }
    let mut components = importer.as_str().split('/').collect::<Vec<_>>();
    let _ = components.pop();
    for component in specifier.split('/') {
        match component {
            "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err(InvalidModuleSpecifier);
                }
            }
            "" => return Err(InvalidModuleSpecifier),
            value => components.push(value),
        }
    }
    let resolved = components.join("/");
    if !has_exact_zry_extension(&resolved) {
        return Err(InvalidModuleSpecifier);
    }
    NormalizedSourcePath::new(resolved).map_err(|_| InvalidModuleSpecifier)
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn has_exact_zry_extension(value: &str) -> bool {
    value.ends_with(".zry")
}

/// A module specifier failed the frozen portable explicit-relative grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidModuleSpecifier;

impl fmt::Display for InvalidModuleSpecifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid explicit relative .zry module specifier")
    }
}

impl std::error::Error for InvalidModuleSpecifier {}

impl<'de> Deserialize<'de> for NormalizedSourcePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let path = String::deserialize(deserializer)?;
        Self::new(path).map_err(de::Error::custom)
    }
}

impl fmt::Display for NormalizedSourcePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Half-open UTF-8 byte range in a source file.
///
/// Values can be constructed only by a [`SourceMap`], which proves file identity, bounds, and
/// character boundaries. Provider wire data uses [`UntrustedSpan`] instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    file: FileId,
    start: u32,
    end: u32,
}

impl Span {
    /// Returns the file containing this range.
    #[must_use]
    pub const fn file(self) -> FileId {
        self.file
    }

    /// Returns the inclusive UTF-8 byte offset.
    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }

    /// Returns the exclusive UTF-8 byte offset.
    #[must_use]
    pub const fn end(self) -> u32 {
        self.end
    }
}

/// Untrusted half-open UTF-8 byte range received from a wire provider.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UntrustedSpan {
    /// Untrusted raw file identifier.
    pub file: u32,
    /// Untrusted inclusive UTF-8 byte offset.
    pub start: u32,
    /// Untrusted exclusive UTF-8 byte offset.
    pub end: u32,
}

/// One unverified source-map input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFileInput {
    /// Portable workspace-relative path.
    pub path: String,
    /// UTF-8 source text, preserved byte-for-byte.
    pub text: String,
}

/// One immutable source file owned by a [`SourceMap`].
#[derive(Clone, Debug)]
pub struct SourceFile {
    id: FileId,
    path: NormalizedSourcePath,
    text: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Returns this file's map-local identity.
    #[must_use]
    pub const fn id(&self) -> FileId {
        self.id
    }

    /// Returns this file's normalized path.
    #[must_use]
    pub const fn path(&self) -> &NormalizedSourcePath {
        &self.path
    }

    /// Returns the original UTF-8 text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Immutable authority for a bounded, normalized set of source files.
#[derive(Clone, Debug)]
pub struct SourceMap {
    id: u64,
    files: Vec<SourceFile>,
    identities: BTreeMap<String, FileId>,
}

impl SourceMap {
    /// Returns the opaque identity used to bind sealed compiler snapshots to this source map.
    #[must_use]
    pub const fn identity(&self) -> SourceMapIdentity {
        SourceMapIdentity(self.id)
    }

    /// Builds a source map atomically and assigns stable dense identifiers by path order.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic validation error after sorting inputs by raw path.
    pub fn build(mut inputs: Vec<SourceFileInput>) -> Result<Self, SourceError> {
        if inputs.len() > MAX_SOURCE_FILES {
            return Err(SourceError::new(
                "ZRYNA-S1002",
                None,
                format!(
                    "source map contains {} files; the limit is {MAX_SOURCE_FILES}",
                    inputs.len()
                ),
                "reduce the source set before analysis",
            ));
        }

        inputs.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
        let mut normalized = Vec::with_capacity(inputs.len());
        for input in inputs {
            let path = NormalizedSourcePath::new(input.path)?;
            if input.text.len() > MAX_SOURCE_FILE_BYTES {
                return Err(SourceError::new(
                    "ZRYNA-S1003",
                    Some(path.as_str().to_owned()),
                    format!(
                        "source file contains {} UTF-8 bytes; the limit is {MAX_SOURCE_FILE_BYTES}",
                        input.text.len()
                    ),
                    "split or reduce the source file before analysis",
                ));
            }
            normalized.push((path, input.text));
        }
        normalized.sort_by(|left, right| left.0.cmp(&right.0));

        let source_map_id = NEXT_SOURCE_MAP_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| current.checked_add(1))
            .map_err(|_| {
                SourceError::new(
                    "ZRYNA-S1010",
                    None,
                    "source-map identity space is exhausted",
                    "restart the compiler process before creating another source map",
                )
            })?;
        let mut total_bytes = 0_usize;
        let mut identities = BTreeMap::new();
        let mut files: Vec<SourceFile> = Vec::with_capacity(normalized.len());
        for (path, text) in normalized {
            let identity = path.portable_identity();
            if let Some(existing) =
                identities.get(&identity).and_then(|id: &FileId| files.get(id.index as usize))
            {
                return Err(SourceError::new(
                    "ZRYNA-S1004",
                    Some(path.as_str().to_owned()),
                    format!(
                        "source path collides with '{}' under the portable path identity",
                        existing.path()
                    ),
                    "use source paths that remain unique when ASCII case is ignored",
                ));
            }
            total_bytes = total_bytes.checked_add(text.len()).ok_or_else(|| {
                SourceError::new(
                    "ZRYNA-S1005",
                    None,
                    "aggregate source size overflowed the supported range",
                    "reduce the source set before analysis",
                )
            })?;
            if total_bytes > MAX_SOURCE_BYTES {
                return Err(SourceError::new(
                    "ZRYNA-S1005",
                    None,
                    format!(
                        "source map contains {total_bytes} UTF-8 bytes; the limit is {MAX_SOURCE_BYTES}"
                    ),
                    "reduce the source set before analysis",
                ));
            }
            let raw_id = u32::try_from(files.len()).map_err(|_| {
                SourceError::new(
                    "ZRYNA-S1002",
                    None,
                    "source file count exceeds the supported identifier range",
                    "reduce the source set before analysis",
                )
            })?;
            let id = FileId { source_map_id, index: raw_id };
            let line_starts = line_starts(&text)?;
            identities.insert(identity, id);
            files.push(SourceFile { id, path, text, line_starts });
        }

        Ok(Self { id: source_map_id, files, identities })
    }

    /// Returns the number of authoritative files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns whether the map contains no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Finds a file by a previously normalized path.
    #[must_use]
    pub fn file_id(&self, path: &NormalizedSourcePath) -> Option<FileId> {
        self.identities.get(&path.portable_identity()).copied()
    }

    /// Finds a source file by map-local identity.
    #[must_use]
    pub fn source(&self, id: FileId) -> Option<&SourceFile> {
        if id.source_map_id != self.id {
            return None;
        }
        usize::try_from(id.index).ok().and_then(|index| self.files.get(index))
    }

    /// Validates a raw dense identifier against this map.
    ///
    /// # Errors
    ///
    /// Returns a source error when `raw` was not issued by this source map.
    pub fn verify_file_id(&self, raw: u32) -> Result<FileId, SourceError> {
        let file = FileId { source_map_id: self.id, index: raw };
        self.source(file).map(SourceFile::id).ok_or_else(|| unknown_file(file))
    }

    /// Constructs and validates an authoritative UTF-8 byte span.
    ///
    /// # Errors
    ///
    /// Returns a source error for an unknown file, reversed range, out-of-bounds offset,
    /// or offset inside a UTF-8 code point.
    pub fn span(&self, file: FileId, start: u32, end: u32) -> Result<Span, SourceError> {
        let span = Span { file, start, end };
        self.resolve(span)?;
        Ok(span)
    }

    /// Validates an untrusted provider range and returns an authoritative span.
    ///
    /// # Errors
    ///
    /// Returns a source error for an unknown identity, reversed or out-of-bounds range, or an
    /// endpoint inside a UTF-8 code point.
    pub fn verify_span(&self, span: UntrustedSpan) -> Result<Span, SourceError> {
        self.span(FileId { source_map_id: self.id, index: span.file }, span.start, span.end)
    }

    /// Converts exact UTF-16 code-unit boundaries into an authoritative UTF-8 byte span.
    ///
    /// # Errors
    ///
    /// Returns a source error when either offset is outside the file or splits a surrogate pair.
    pub fn span_from_utf16(&self, file: FileId, start: u32, end: u32) -> Result<Span, SourceError> {
        if start > end {
            return Err(invalid_span_order(file, start, end));
        }
        let source = self.source(file).ok_or_else(|| unknown_file(file))?;
        let start = utf16_to_utf8(source, start)?;
        let end = utf16_to_utf8(source, end)?;
        self.span(file, start, end)
    }

    /// Resolves and revalidates a span against this exact source authority.
    ///
    /// # Errors
    ///
    /// Returns a source error if the file or range is not valid for this map.
    pub fn resolve(&self, span: Span) -> Result<ResolvedSpan<'_>, SourceError> {
        let source = self.source(span.file).ok_or_else(|| unknown_file(span.file))?;
        if span.start > span.end {
            return Err(invalid_span_order(span.file, span.start, span.end));
        }
        let source_len = u32::try_from(source.text.len()).map_err(|_| {
            SourceError::new(
                "ZRYNA-S1007",
                Some(source.path.as_str().to_owned()),
                "source length exceeds the supported offset range",
                "reduce the source file before analysis",
            )
        })?;
        if span.end > source_len {
            return Err(SourceError::new(
                "ZRYNA-S1007",
                Some(source.path.as_str().to_owned()),
                format!(
                    "source span {}..{} exceeds file length {source_len}",
                    span.start, span.end
                ),
                "keep source ranges within the referenced file",
            ));
        }
        let start =
            usize::try_from(span.start).map_err(|_| invalid_boundary(source, span.start))?;
        let end = usize::try_from(span.end).map_err(|_| invalid_boundary(source, span.end))?;
        if !source.text.is_char_boundary(start) {
            return Err(invalid_boundary(source, span.start));
        }
        if !source.text.is_char_boundary(end) {
            return Err(invalid_boundary(source, span.end));
        }
        Ok(ResolvedSpan {
            source,
            start: resolve_position(source, span.start),
            end: resolve_position(source, span.end),
        })
    }
}

/// A zero-based resolved position whose byte offset remains authoritative.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedPosition {
    /// UTF-8 byte offset from the beginning of the file.
    pub byte_offset: u32,
    /// Zero-based line number; CRLF counts as one line break at LF.
    pub line: u32,
    /// Zero-based Unicode-scalar column.
    pub scalar_column: u32,
    /// Zero-based UTF-16 code-unit column for editor interoperability.
    pub utf16_column: u32,
}

/// A span proven to belong to one source map.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedSpan<'source> {
    source: &'source SourceFile,
    /// Resolved inclusive start.
    pub start: ResolvedPosition,
    /// Resolved exclusive end.
    pub end: ResolvedPosition,
}

impl<'source> ResolvedSpan<'source> {
    /// Returns the authoritative source file.
    #[must_use]
    pub const fn source(self) -> &'source SourceFile {
        self.source
    }
}

/// Stable source validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceError {
    code: &'static str,
    path: Option<String>,
    message: String,
    guidance: &'static str,
}

impl SourceError {
    fn new(
        code: &'static str,
        path: Option<String>,
        message: impl Into<String>,
        guidance: &'static str,
    ) -> Self {
        Self { code, path, message: message.into(), guidance }
    }

    /// Returns the stable public error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the affected source path, when one is known.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the short problem statement.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns concrete remediation guidance.
    #[must_use]
    pub const fn guidance(&self) -> &'static str {
        self.guidance
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = escape_text(&self.message);
        let guidance = escape_text(self.guidance);
        if let Some(path) = &self.path {
            write!(formatter, "{} [{}] {}: {}", self.code, escape_text(path), message, guidance)
        } else {
            write!(formatter, "{}: {message}: {guidance}", self.code)
        }
    }
}

impl std::error::Error for SourceError {}

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

fn validate_source_path(path: &str) -> Result<(), SourceError> {
    let invalid = |message: String| {
        SourceError::new(
            "ZRYNA-S1001",
            Some(path.to_owned()),
            message,
            "use a portable ASCII workspace-relative path with '/' separators",
        )
    };
    if path.is_empty() {
        return Err(invalid("source path is empty".to_owned()));
    }
    if path.len() > MAX_SOURCE_PATH_BYTES {
        return Err(invalid(format!(
            "source path contains {} bytes; the limit is {MAX_SOURCE_PATH_BYTES}",
            path.len()
        )));
    }
    if !path.is_ascii() {
        return Err(invalid("source path contains non-ASCII characters".to_owned()));
    }
    if path.starts_with('/') || path.starts_with("//") {
        return Err(invalid("source path is absolute".to_owned()));
    }
    if path.contains('\\') {
        return Err(invalid("source path contains a backslash".to_owned()));
    }
    let components: Vec<_> = path.split('/').collect();
    if components.len() > MAX_SOURCE_PATH_COMPONENTS {
        return Err(invalid(format!(
            "source path contains {} components; the limit is {MAX_SOURCE_PATH_COMPONENTS}",
            components.len()
        )));
    }
    for component in components {
        if component.is_empty() {
            return Err(invalid("source path contains an empty component".to_owned()));
        }
        if matches!(component, "." | "..") {
            return Err(invalid("source path contains a traversal component".to_owned()));
        }
        if component.len() > 255 {
            return Err(invalid("source path component exceeds 255 bytes".to_owned()));
        }
        if component.ends_with(['.', ' ']) {
            return Err(invalid("source path component ends with a dot or space".to_owned()));
        }
        if component.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(invalid("source path contains a control character".to_owned()));
        }
        if component.contains(['<', '>', ':', '"', '|', '?', '*']) {
            return Err(invalid("source path contains a Windows-reserved character".to_owned()));
        }
        let stem = component.split('.').next().unwrap_or_default();
        let folded = stem.to_ascii_lowercase();
        let numbered_device =
            folded.strip_prefix("com").or_else(|| folded.strip_prefix("lpt")).is_some_and(
                |suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"),
            );
        if matches!(folded.as_str(), "con" | "prn" | "aux" | "nul") || numbered_device {
            return Err(invalid("source path contains a Windows-reserved device name".to_owned()));
        }
    }
    Ok(())
}

fn line_starts(text: &str) -> Result<Vec<u32>, SourceError> {
    let mut starts = vec![0];
    let bytes = text.as_bytes();
    let mut index = 0_usize;
    while index < bytes.len() {
        let line_end = match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => Some(index + 2),
            b'\r' | b'\n' => Some(index + 1),
            _ => None,
        };
        if let Some(line_end) = line_end {
            let next = u32::try_from(line_end).map_err(|_| {
                SourceError::new(
                    "ZRYNA-S1003",
                    None,
                    "source line offset exceeds the supported range",
                    "reduce the source file before analysis",
                )
            })?;
            starts.push(next);
            index = line_end;
        } else {
            index += 1;
        }
    }
    Ok(starts)
}

fn resolve_position(source: &SourceFile, offset: u32) -> ResolvedPosition {
    let line_index = source.line_starts.partition_point(|start| *start <= offset).saturating_sub(1);
    let line_start = source.line_starts[line_index];
    let prefix = &source.text[line_start as usize..offset as usize];
    ResolvedPosition {
        byte_offset: offset,
        line: u32::try_from(line_index).unwrap_or(u32::MAX),
        scalar_column: u32::try_from(prefix.chars().count()).unwrap_or(u32::MAX),
        utf16_column: u32::try_from(prefix.encode_utf16().count()).unwrap_or(u32::MAX),
    }
}

fn utf16_to_utf8(source: &SourceFile, requested: u32) -> Result<u32, SourceError> {
    let mut utf16_offset = 0_u32;
    for (byte_offset, character) in source.text.char_indices() {
        if requested == utf16_offset {
            return u32::try_from(byte_offset).map_err(|_| invalid_utf16(source, requested));
        }
        let utf16_width = if character.len_utf16() == 1 { 1 } else { 2 };
        let next = utf16_offset
            .checked_add(utf16_width)
            .ok_or_else(|| invalid_utf16(source, requested))?;
        if requested < next {
            return Err(invalid_utf16(source, requested));
        }
        utf16_offset = next;
    }
    if requested == utf16_offset {
        return u32::try_from(source.text.len()).map_err(|_| invalid_utf16(source, requested));
    }
    Err(invalid_utf16(source, requested))
}

fn unknown_file(file: FileId) -> SourceError {
    SourceError::new(
        "ZRYNA-S1006",
        None,
        format!("source file identifier {} is not present in this source map", file.index),
        "use a file identifier issued by the same source map",
    )
}

fn invalid_span_order(file: FileId, start: u32, end: u32) -> SourceError {
    SourceError::new(
        "ZRYNA-S1007",
        None,
        format!("source span {start}..{end} in file {} starts after it ends", file.index),
        "ensure every source range is half-open and ordered",
    )
}

fn invalid_boundary(source: &SourceFile, offset: u32) -> SourceError {
    SourceError::new(
        "ZRYNA-S1008",
        Some(source.path.as_str().to_owned()),
        format!("UTF-8 byte offset {offset} is not a character boundary"),
        "use exact UTF-8 byte boundaries from the authoritative source text",
    )
}

fn invalid_utf16(source: &SourceFile, offset: u32) -> SourceError {
    SourceError::new(
        "ZRYNA-S1009",
        Some(source.path.as_str().to_owned()),
        format!("UTF-16 code-unit offset {offset} is not an exact source boundary"),
        "use an in-range UTF-16 offset that does not split a surrogate pair",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(path: &str, text: &str) -> SourceFileInput {
        SourceFileInput { path: path.to_owned(), text: text.to_owned() }
    }

    fn map(inputs: Vec<SourceFileInput>) -> SourceMap {
        SourceMap::build(inputs).expect("fixture source map must be valid")
    }

    #[test]
    fn stable_ids_do_not_depend_on_input_order() {
        let forward = map(vec![input("src/z.zry", "z"), input("src/a.zry", "a")]);
        let reverse = map(vec![input("src/a.zry", "a"), input("src/z.zry", "z")]);
        for path in ["src/a.zry", "src/z.zry"] {
            let path = NormalizedSourcePath::new(path).expect("fixture path must be valid");
            assert_eq!(
                forward.file_id(&path).map(FileId::index),
                reverse.file_id(&path).map(FileId::index)
            );
        }
    }

    #[test]
    fn portable_path_policy_rejects_unsafe_and_colliding_paths() {
        for path in [
            "",
            "/root.zry",
            "C:/root.zry",
            "//server/share.zry",
            "src\\a.zry",
            "src//a.zry",
            "src/./a.zry",
            "src/../a.zry",
            "src/con.zry",
            "src/a?.zry",
            "src/trailing. ",
            "src/é.zry",
        ] {
            let error =
                SourceMap::build(vec![input(path, "")]).expect_err("unsafe source path must fail");
            assert_eq!(error.code(), "ZRYNA-S1001", "path: {path:?}");
        }

        let first = SourceMap::build(vec![input("src/A.zry", ""), input("src/a.zry", "")])
            .expect_err("portable collision must fail");
        let second = SourceMap::build(vec![input("src/a.zry", ""), input("src/A.zry", "")])
            .expect_err("portable collision must fail");
        assert_eq!(first, second);
        assert_eq!(first.code(), "ZRYNA-S1004");
    }

    #[test]
    fn source_error_display_escapes_an_invalid_path() {
        let error = SourceMap::build(vec![input("src/spoof\npath.zry", "")])
            .expect_err("control characters in a source path must fail");

        assert_eq!(
            error.to_string(),
            "ZRYNA-S1001 [src/spoof\\npath.zry] source path contains a control character: use a portable ASCII workspace-relative path with '/' separators"
        );
    }

    #[test]
    fn span_validation_rejects_forged_ranges_and_accepts_eof() {
        let sources = map(vec![input("src/main.zry", "aé😀")]);
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");

        assert_eq!(sources.span(file, 7, 7).expect("EOF span must be valid").end(), 7);
        assert_eq!(
            sources.span(file, 3, 2).expect_err("reversed span must fail").code(),
            "ZRYNA-S1007"
        );
        assert_eq!(sources.span(file, 0, 8).expect_err("past EOF must fail").code(), "ZRYNA-S1007");
        assert_eq!(
            sources.span(file, 2, 3).expect_err("mid-code-point span must fail").code(),
            "ZRYNA-S1008"
        );
        let forged = UntrustedSpan { file: u32::MAX, start: 0, end: 0 };
        assert_eq!(
            sources.verify_span(forged).expect_err("unknown id must fail").code(),
            "ZRYNA-S1006"
        );
    }

    #[test]
    fn utf16_conversion_handles_bmp_astral_crlf_and_combining_text() {
        let sources = map(vec![input("src/main.zry", "a😀b\r\né\u{301}")]);
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = sources.file_id(&path).expect("fixture file must exist");

        let emoji = sources.span_from_utf16(file, 1, 3).expect("emoji boundaries must convert");
        assert_eq!((emoji.start(), emoji.end()), (1, 5));
        assert_eq!(
            sources.span_from_utf16(file, 2, 3).expect_err("surrogate split must fail").code(),
            "ZRYNA-S1009"
        );
        let second_line = sources.span_from_utf16(file, 6, 7).expect("BMP range must convert");
        let resolved = sources.resolve(second_line).expect("converted span must resolve");
        assert_eq!(
            (resolved.start.byte_offset, resolved.start.line, resolved.start.scalar_column),
            (8, 1, 0)
        );
        assert_eq!((resolved.end.byte_offset, resolved.end.utf16_column), (10, 1));
    }

    #[test]
    fn wire_spans_stay_untrusted_until_source_map_verification() {
        let sources = map(vec![input("src/main.zry", "x")]);
        let reversed = serde_json::from_str::<UntrustedSpan>(r#"{"file":0,"start":2,"end":1}"#)
            .expect("wire shape must deserialize before source validation");
        assert_eq!(
            sources.verify_span(reversed).expect_err("reversed span must fail").code(),
            "ZRYNA-S1007"
        );
        let unknown =
            serde_json::from_str::<UntrustedSpan>(r#"{"file":0,"start":0,"end":1,"extra":true}"#);
        assert!(unknown.is_err());
    }

    #[test]
    fn spans_are_bound_to_the_exact_source_map() {
        let first = map(vec![input("src/main.zry", "same")]);
        let second = map(vec![input("src/main.zry", "same")]);
        let path = NormalizedSourcePath::new("src/main.zry").expect("fixture path must be valid");
        let file = first.file_id(&path).expect("fixture file must exist");
        let span = first.span(file, 0, 4).expect("fixture span must be valid");
        assert_eq!(
            second.resolve(span).expect_err("cross-map span must fail").code(),
            "ZRYNA-S1006"
        );
    }

    #[test]
    fn source_map_limits_accept_exact_boundaries_and_reject_one_more() {
        let exact_files =
            (0..MAX_SOURCE_FILES).map(|index| input(&format!("src/f{index:04}.zry"), "")).collect();
        assert_eq!(map(exact_files).len(), MAX_SOURCE_FILES);
        let too_many = (0..=MAX_SOURCE_FILES)
            .map(|index| input(&format!("src/f{index:04}.zry"), ""))
            .collect();
        assert_eq!(
            SourceMap::build(too_many).expect_err("file-count overflow must fail").code(),
            "ZRYNA-S1002"
        );

        let exact_file = "x".repeat(MAX_SOURCE_FILE_BYTES);
        assert_eq!(map(vec![input("src/exact.zry", &exact_file)]).len(), 1);
        let too_large = "x".repeat(MAX_SOURCE_FILE_BYTES + 1);
        assert_eq!(
            SourceMap::build(vec![input("src/large.zry", &too_large)])
                .expect_err("file-size overflow must fail")
                .code(),
            "ZRYNA-S1003"
        );

        let chunk = "x".repeat(MAX_SOURCE_FILE_BYTES);
        let exact_total = (0..(MAX_SOURCE_BYTES / MAX_SOURCE_FILE_BYTES))
            .map(|index| input(&format!("src/chunk{index:02}.zry"), &chunk))
            .collect();
        assert_eq!(map(exact_total).len(), MAX_SOURCE_BYTES / MAX_SOURCE_FILE_BYTES);
        let over_total = (0..=(MAX_SOURCE_BYTES / MAX_SOURCE_FILE_BYTES))
            .map(|index| input(&format!("src/chunk{index:02}.zry"), &chunk))
            .collect();
        assert_eq!(
            SourceMap::build(over_total).expect_err("aggregate overflow must fail").code(),
            "ZRYNA-S1005"
        );
    }

    #[test]
    fn explicit_zry_import_resolution_is_portable_and_cannot_escape() {
        let importer = NormalizedSourcePath::new("src/nested/main.zry")
            .expect("fixture importer must normalize");
        for (specifier, expected) in [
            ("./dep.zry", "src/nested/dep.zry"),
            ("./child/../dep.zry", "src/nested/dep.zry"),
            ("../shared.zry", "src/shared.zry"),
            ("../../root.zry", "root.zry"),
        ] {
            assert_eq!(
                resolve_explicit_zry_import(&importer, specifier)
                    .expect("portable specifier must resolve")
                    .as_str(),
                expected
            );
        }
        for rejected in [
            "",
            "dep.zry",
            "/dep.zry",
            "C:/dep.zry",
            "//server/dep.zry",
            "https://example.invalid/dep.zry",
            "./dep",
            "./dep.ZRY",
            "./dep.zry?query",
            "./dep.zry#fragment",
            ".\\dep.zry",
            "../../../escape.zry",
        ] {
            assert!(
                resolve_explicit_zry_import(&importer, rejected).is_err(),
                "specifier must be rejected: {rejected}"
            );
        }
    }
}
