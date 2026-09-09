use std::collections::BTreeMap;

use zryna_diagnostics::Diagnostic;
use zryna_source::{
    MAX_SOURCE_FILE_BYTES, MAX_SOURCE_FILES, NormalizedSourcePath, SourceFileInput, SourceMap,
};

use super::{LexError, LexedProject, MAX_SOURCE_BYTES_PER_PROJECT, lex, raw_error, source_error};

/// One untrusted native-frontend source before UTF-8 admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSourceBytes {
    /// Portable workspace-relative path claimed by the input.
    pub path: String,
    /// Exact untrusted source bytes.
    pub bytes: Vec<u8>,
}

/// An exact byte range before a [`SourceMap`] authority exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawByteSpan {
    file: u32,
    path: NormalizedSourcePath,
    start: u32,
    end: u32,
}

impl RawByteSpan {
    /// Returns the canonical dense input index after portable path ordering.
    #[must_use]
    pub const fn file(&self) -> u32 {
        self.file
    }

    /// Returns the validated portable path of the exact input file.
    #[must_use]
    pub const fn path(&self) -> &NormalizedSourcePath {
        &self.path
    }

    /// Returns the inclusive raw byte offset.
    #[must_use]
    pub const fn start(&self) -> u32 {
        self.start
    }

    /// Returns the exclusive raw byte offset.
    #[must_use]
    pub const fn end(&self) -> u32 {
        self.end
    }
}

/// A newly authenticated source map retained with its bound lexical result.
#[derive(Clone, Debug)]
pub struct AdmittedLexedProject {
    sources: SourceMap,
    project: LexedProject,
}

impl AdmittedLexedProject {
    /// Returns the authoritative source map created from the admitted bytes.
    #[must_use]
    pub const fn sources(&self) -> &SourceMap {
        &self.sources
    }

    /// Returns the lexical project bound to [`Self::sources`].
    #[must_use]
    pub const fn project(&self) -> &LexedProject {
        &self.project
    }
}

struct CheckedInput {
    path: NormalizedSourcePath,
    bytes: Vec<u8>,
}

/// Authenticates bounded raw source bytes and lexes the resulting exact [`SourceMap`].
///
/// Size limits are enforced before UTF-8 conversion. Invalid encoding is reported with a
/// [`RawByteSpan`] because no source-map-authenticated [`zryna_source::Span`] exists yet.
///
/// # Errors
///
/// Returns [`LexError`] atomically for an invalid path, duplicate portable identity, file-count or
/// byte budget, malformed UTF-8, source-map construction failure, or lexical resource failure.
pub fn admit_and_lex(mut inputs: Vec<NativeSourceBytes>) -> Result<AdmittedLexedProject, LexError> {
    if inputs.len() > MAX_SOURCE_FILES {
        return Err(global_source_error(
            "ZRYNA-S1002",
            "source map contains too many files",
            "reduce the number of source files before analysis",
        ));
    }
    inputs.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));

    let mut identities = BTreeMap::<String, NormalizedSourcePath>::new();
    let mut checked = Vec::with_capacity(inputs.len());
    for input in inputs {
        let path = NormalizedSourcePath::new(input.path).map_err(|error| source_error(&error))?;
        let identity = path.portable_identity();
        if let Some(existing) = identities.insert(identity, path.clone()) {
            return Err(LexError {
                diagnostic: Diagnostic::error(
                    "ZRYNA-S1004",
                    Some(path.as_str().to_owned()),
                    format!(
                        "source path collides with '{existing}' under the portable path identity"
                    ),
                    "use source paths that remain unique when ASCII case is ignored",
                ),
                raw_byte_span: None,
            });
        }
        checked.push(CheckedInput { path, bytes: input.bytes });
    }

    preflight_bytes(&checked)?;
    let mut decoded = Vec::with_capacity(checked.len());
    for (file, input) in checked.into_iter().enumerate() {
        let file = u32::try_from(file).map_err(|_| {
            global_source_error(
                "ZRYNA-S1002",
                "source file identifier exceeds its supported range",
                "reduce the number of source files before analysis",
            )
        })?;
        let path = input.path;
        let text = String::from_utf8(input.bytes).map_err(|error| {
            let invalid = error.utf8_error();
            let start = invalid.valid_up_to();
            let end = invalid.error_len().map_or(error.as_bytes().len(), |length| start + length);
            raw_error(
                "ZRYNA-F1501",
                raw_span(file, path.clone(), start, end),
                "source bytes are not valid UTF-8",
                "provide well-formed UTF-8 source bytes",
            )
        })?;
        decoded.push(SourceFileInput { path: path.as_str().to_owned(), text });
    }

    let sources = SourceMap::build(decoded).map_err(|error| source_error(&error))?;
    let project = lex(&sources)?;
    debug_assert!(project.is_bound_to(&sources));
    Ok(AdmittedLexedProject { sources, project })
}

fn preflight_bytes(inputs: &[CheckedInput]) -> Result<(), LexError> {
    let mut total = 0_usize;
    for (file, input) in inputs.iter().enumerate() {
        let file = u32::try_from(file).map_err(|_| {
            global_source_error(
                "ZRYNA-S1002",
                "source file identifier exceeds its supported range",
                "reduce the number of source files before analysis",
            )
        })?;
        if input.bytes.len() > MAX_SOURCE_FILE_BYTES {
            return Err(raw_error(
                "ZRYNA-F1502",
                raw_span(
                    file,
                    input.path.clone(),
                    MAX_SOURCE_FILE_BYTES,
                    MAX_SOURCE_FILE_BYTES + 1,
                ),
                "source file bytes exceed the source-map limit",
                "reduce the bounded source before native lexing",
            ));
        }
        let remaining = MAX_SOURCE_BYTES_PER_PROJECT.saturating_sub(total);
        if input.bytes.len() > remaining {
            return Err(raw_error(
                "ZRYNA-F1502",
                raw_span(file, input.path.clone(), remaining, remaining + 1),
                "project source bytes exceed the protocol-v4 limit",
                "reduce the bounded source before native lexing",
            ));
        }
        total = total.checked_add(input.bytes.len()).ok_or_else(|| {
            global_source_error(
                "ZRYNA-F1502",
                "project source byte inventory overflowed",
                "reduce the bounded source before native lexing",
            )
        })?;
    }
    Ok(())
}

fn raw_span(file: u32, path: NormalizedSourcePath, start: usize, end: usize) -> RawByteSpan {
    RawByteSpan {
        file,
        path,
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
    }
}

fn global_source_error(
    code: &'static str,
    message: &'static str,
    guidance: &'static str,
) -> LexError {
    LexError { diagnostic: Diagnostic::error(code, None, message, guidance), raw_byte_span: None }
}
