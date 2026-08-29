//! Fail-closed executable syntax protocol version 2.

use std::{collections::BTreeSet, fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, IgnoredAny, SeqAccess, Visitor},
};
use zryna_diagnostics::{Diagnostic, PrimaryLocation, Severity};
use zryna_source::{
    FileId, MAX_SOURCE_FILES, NormalizedSourcePath, SourceMap, Span, UntrustedSpan,
};

/// Exact protocol version represented by this module.
pub const PROTOCOL_VERSION: u32 = 2;
/// Maximum serialized bytes accepted from one provider response.
pub const MAX_RESPONSE_BYTES: usize = 16 * 1_024 * 1_024;
/// Maximum exported functions accepted in one source file.
pub const MAX_FUNCTIONS_PER_FILE: usize = 4_096;
/// Maximum exported functions accepted in one project.
pub const MAX_FUNCTIONS_PER_PROJECT: usize = 16_384;
/// Maximum parameters accepted in one function.
pub const MAX_PARAMETERS_PER_FUNCTION: usize = 256;
/// Maximum parameters accepted in one project.
pub const MAX_PARAMETERS_PER_PROJECT: usize = 262_144;
/// Maximum return statements accepted in one function body.
pub const MAX_STATEMENTS_PER_FUNCTION: usize = 4_096;
/// Maximum return statements accepted in one project.
pub const MAX_STATEMENTS_PER_PROJECT: usize = 65_536;
/// Maximum expression arena entries accepted in one function.
pub const MAX_EXPRESSIONS_PER_FUNCTION: usize = 16_384;
/// Maximum expression arena entries accepted in one project.
pub const MAX_EXPRESSIONS_PER_PROJECT: usize = 262_144;
/// Maximum expression-tree depth accepted after iterative arena verification.
pub const MAX_EXPRESSION_DEPTH: u32 = 128;
/// Maximum Unicode scalar values accepted in one identifier or type spelling.
pub const MAX_NAME_CHARACTERS: usize = 1_024;
/// Maximum UTF-8 bytes accepted in one integer-literal spelling.
pub const MAX_LITERAL_BYTES: usize = 64;
/// Maximum provider diagnostics accepted in one snapshot.
pub const MAX_PROVIDER_DIAGNOSTICS: usize = 256;
/// Maximum Unicode scalar values accepted in one diagnostic message or guidance value.
pub const MAX_DIAGNOSTIC_TEXT_CHARACTERS: usize = 4_096;
/// Maximum structural validation diagnostics retained for one snapshot.
pub const MAX_VALIDATION_ERRORS: usize = 256;

/// Complete untrusted protocol-v2 response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProjectSyntaxSnapshot {
    /// Exact protocol schema version.
    pub schema_version: u32,
    /// Claimed complete source-file set.
    #[serde(deserialize_with = "deserialize_files")]
    pub files: Vec<RawSourceUnit>,
    /// Bounded provider diagnostics outside Zryna semantic authority.
    #[serde(deserialize_with = "deserialize_diagnostics")]
    pub diagnostics: Vec<RawProviderDiagnostic>,
}

/// One untrusted source unit.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawSourceUnit {
    /// Claimed snapshot-local file identifier.
    pub id: u32,
    /// Claimed normalized source path.
    pub path: String,
    /// Exported functions in this file.
    #[serde(deserialize_with = "deserialize_functions")]
    pub functions: Vec<RawFunctionSyntax>,
}

/// One untrusted exported function.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionSyntax {
    /// Full declaration span.
    pub span: UntrustedSpan,
    /// Export keyword span.
    pub export_span: UntrustedSpan,
    /// Function keyword span.
    pub function_span: UntrustedSpan,
    /// Declared function name.
    pub name: RawIdentifierSyntax,
    /// Source-ordered parameters.
    #[serde(deserialize_with = "deserialize_parameters")]
    pub parameters: Vec<RawParameterSyntax>,
    /// Explicit or missing result annotation.
    pub result_type: RawTypeSyntax,
    /// Executable function body.
    pub body: RawFunctionBodySyntax,
}

/// One untrusted identifier and its exact source range.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawIdentifierSyntax {
    /// Provider-neutral identifier spelling.
    pub text: String,
    /// Identifier source span.
    pub span: UntrustedSpan,
}

/// One untrusted function parameter.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawParameterSyntax {
    /// Full parameter span.
    pub span: UntrustedSpan,
    /// Parameter name.
    pub name: RawIdentifierSyntax,
    /// Explicit or missing annotation.
    pub type_syntax: RawTypeSyntax,
}

/// One untrusted type annotation.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawTypeSyntax {
    /// Annotation span, including the insertion point when missing.
    pub span: UntrustedSpan,
    /// Provider-neutral annotation form.
    pub kind: RawTypeSyntaxKind,
}

/// Provider-neutral untrusted type annotation forms.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawTypeSyntaxKind {
    /// No annotation was written; strict semantics will reject it when required.
    Missing,
    /// Named spelling such as `i32`, `bool`, or unsupported `any`.
    Named {
        /// Exact normalized type spelling retained for semantic analysis.
        name: String,
    },
}

/// One untrusted executable function body.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionBodySyntax {
    /// Full body span.
    pub span: UntrustedSpan,
    /// Source-ordered statements.
    #[serde(deserialize_with = "deserialize_statements")]
    pub statements: Vec<RawStatementSyntax>,
    /// Canonical postorder flat expression arena.
    #[serde(deserialize_with = "deserialize_expressions")]
    pub expressions: Vec<RawExpressionSyntax>,
}

/// One untrusted statement.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawStatementSyntax {
    /// Full statement span.
    pub span: UntrustedSpan,
    /// Provider-neutral statement operation.
    pub kind: RawStatementKind,
}

/// Provider-neutral statement operations in protocol v2.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawStatementKind {
    /// A value-returning statement.
    Return {
        /// `return` keyword span.
        keyword_span: UntrustedSpan,
        /// Expression-arena index returned by this statement.
        value: u32,
    },
}

/// One untrusted flat-arena expression.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawExpressionSyntax {
    /// Full expression span.
    pub span: UntrustedSpan,
    /// Provider-neutral expression operation.
    pub kind: RawExpressionKind,
}

/// Provider-neutral expression operations in protocol v2.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawExpressionKind {
    /// Parameter or future local reference.
    Reference {
        /// Referenced source name.
        name: RawIdentifierSyntax,
    },
    /// Boolean literal.
    BoolLiteral {
        /// Literal value.
        value: bool,
    },
    /// Decimal i32 candidate retained for semantic range checking.
    I32Literal {
        /// Canonical decimal spelling.
        spelling: String,
    },
    /// Source-level addition; semantic analysis selects the exact typed IR operation.
    Addition {
        /// `+` token span.
        operator_span: UntrustedSpan,
        /// Left expression-arena index.
        lhs: u32,
        /// Right expression-arena index.
        rhs: u32,
    },
}

/// One untrusted provider diagnostic.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProviderDiagnostic {
    /// Provider-selected stable code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Required source or global location form.
    pub location: RawDiagnosticLocation,
    /// Short problem statement.
    pub message: String,
    /// Concrete remediation guidance.
    pub guidance: String,
}

/// Untrusted provider diagnostic location.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawDiagnosticLocation {
    /// Locationless provider failure.
    Global,
    /// Provider-selected source range.
    Source {
        /// Untrusted source range.
        span: UntrustedSpan,
    },
}

/// Verified provider-neutral project accepted by semantic analysis.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSyntaxSnapshot {
    schema_version: u32,
    files: Vec<SourceUnit>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectSyntaxSnapshot {
    /// Returns the exact verified protocol version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the complete verified source-file set.
    #[must_use]
    pub fn files(&self) -> &[SourceUnit] {
        &self.files
    }

    /// Returns bounded source-map-verified provider diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns whether every file identity and diagnostic span belongs to this exact source map.
    #[must_use]
    pub fn is_bound_to(&self, sources: &SourceMap) -> bool {
        self.files.len() == sources.len()
            && self.files.iter().all(|file| {
                sources.source(file.id).is_some_and(|source| source.path() == &file.path)
            })
            && self
                .diagnostics
                .iter()
                .filter_map(Diagnostic::primary_span)
                .all(|span| sources.resolve(span).is_ok())
    }
}

/// One verified source unit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceUnit {
    id: FileId,
    path: NormalizedSourcePath,
    functions: Vec<FunctionSyntax>,
}

impl SourceUnit {
    /// Returns the authoritative source identity.
    #[must_use]
    pub const fn id(&self) -> FileId {
        self.id
    }

    /// Returns the authoritative source path.
    #[must_use]
    pub const fn path(&self) -> &NormalizedSourcePath {
        &self.path
    }

    /// Returns source-ordered exported functions.
    #[must_use]
    pub fn functions(&self) -> &[FunctionSyntax] {
        &self.functions
    }
}

/// One verified exported function.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSyntax {
    span: Span,
    export_span: Span,
    function_span: Span,
    name: IdentifierSyntax,
    parameters: Vec<ParameterSyntax>,
    result_type: TypeSyntax,
    body: FunctionBodySyntax,
}

impl FunctionSyntax {
    /// Returns the full declaration span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the export keyword span.
    #[must_use]
    pub const fn export_span(&self) -> Span {
        self.export_span
    }

    /// Returns the function keyword span.
    #[must_use]
    pub const fn function_span(&self) -> Span {
        self.function_span
    }

    /// Returns the verified function name.
    #[must_use]
    pub const fn name(&self) -> &IdentifierSyntax {
        &self.name
    }

    /// Returns source-ordered parameters.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterSyntax] {
        &self.parameters
    }

    /// Returns the result annotation.
    #[must_use]
    pub const fn result_type(&self) -> &TypeSyntax {
        &self.result_type
    }

    /// Returns the executable body.
    #[must_use]
    pub const fn body(&self) -> &FunctionBodySyntax {
        &self.body
    }
}

/// One verified identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierSyntax {
    text: String,
    span: Span,
}

impl IdentifierSyntax {
    /// Returns the normalized provider-neutral spelling.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the authoritative identifier span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// One verified parameter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterSyntax {
    span: Span,
    name: IdentifierSyntax,
    type_syntax: TypeSyntax,
}

impl ParameterSyntax {
    /// Returns the full parameter span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the parameter name.
    #[must_use]
    pub const fn name(&self) -> &IdentifierSyntax {
        &self.name
    }

    /// Returns the parameter annotation.
    #[must_use]
    pub const fn type_syntax(&self) -> &TypeSyntax {
        &self.type_syntax
    }
}

/// One verified type annotation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeSyntax {
    span: Span,
    kind: TypeSyntaxKind,
}

impl TypeSyntax {
    /// Returns the annotation span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the provider-neutral annotation form.
    #[must_use]
    pub const fn kind(&self) -> &TypeSyntaxKind {
        &self.kind
    }
}

/// Verified provider-neutral type annotation forms.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TypeSyntaxKind {
    /// No annotation was present.
    Missing,
    /// A named annotation retained for strict semantic analysis.
    Named {
        /// Exact normalized spelling.
        name: String,
    },
}

/// One verified executable body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionBodySyntax {
    span: Span,
    statements: Vec<StatementSyntax>,
    expressions: Vec<ExpressionSyntax>,
}

impl FunctionBodySyntax {
    /// Returns the full body span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns source-ordered statements.
    #[must_use]
    pub fn statements(&self) -> &[StatementSyntax] {
        &self.statements
    }

    /// Returns the canonical postorder expression arena.
    #[must_use]
    pub fn expressions(&self) -> &[ExpressionSyntax] {
        &self.expressions
    }
}

/// One verified statement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatementSyntax {
    span: Span,
    kind: StatementKind,
}

impl StatementSyntax {
    /// Returns the full statement span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the provider-neutral statement operation.
    #[must_use]
    pub const fn kind(&self) -> &StatementKind {
        &self.kind
    }
}

/// Verified statement operations in protocol v2.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StatementKind {
    /// A value-returning statement.
    Return {
        /// `return` keyword span.
        keyword_span: Span,
        /// Owned expression root.
        value: ExpressionId,
    },
}

/// Opaque expression identity within one verified function body.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ExpressionId(u32);

impl ExpressionId {
    /// Returns the deterministic arena index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// One verified expression.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionSyntax {
    span: Span,
    kind: ExpressionKind,
}

impl ExpressionSyntax {
    /// Returns the authoritative full expression span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the provider-neutral expression operation.
    #[must_use]
    pub const fn kind(&self) -> &ExpressionKind {
        &self.kind
    }
}

/// Verified expression operations in protocol v2.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExpressionKind {
    /// Parameter or future local reference.
    Reference {
        /// Referenced source name.
        name: IdentifierSyntax,
    },
    /// Boolean literal.
    BoolLiteral {
        /// Literal value.
        value: bool,
    },
    /// Decimal i32 candidate retained for semantic range checking.
    I32Literal {
        /// Canonical decimal spelling.
        spelling: String,
    },
    /// Source-level addition.
    Addition {
        /// `+` token span.
        operator_span: Span,
        /// Left child.
        lhs: ExpressionId,
        /// Right child.
        rhs: ExpressionId,
    },
}

/// Stable failure while decoding the bounded wire representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxDecodeError {
    /// Provider bytes exceed the pre-deserialization transport bound.
    ResponseTooLarge {
        /// Received bytes.
        actual: usize,
        /// Maximum accepted bytes.
        limit: usize,
    },
    /// JSON is malformed, incomplete, or violates the strict wire shape.
    InvalidSnapshot,
}

impl SyntaxDecodeError {
    /// Returns the stable public error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ResponseTooLarge { .. } => "ZRYNA-Y1003",
            Self::InvalidSnapshot => "ZRYNA-Y1001",
        }
    }
}

impl fmt::Display for SyntaxDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseTooLarge { actual, limit } => write!(
                formatter,
                "{}: syntax response contains {actual} bytes; the limit is {limit}",
                self.code()
            ),
            Self::InvalidSnapshot => {
                write!(formatter, "{}: invalid protocol-v2 syntax snapshot", self.code())
            }
        }
    }
}

impl std::error::Error for SyntaxDecodeError {}

fn deserialize_files<'de, D>(deserializer: D) -> Result<Vec<RawSourceUnit>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawSourceUnit, MAX_SOURCE_FILES>(deserializer, "source files")
}

fn deserialize_functions<'de, D>(deserializer: D) -> Result<Vec<RawFunctionSyntax>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawFunctionSyntax, MAX_FUNCTIONS_PER_FILE>(deserializer, "functions")
}

fn deserialize_parameters<'de, D>(deserializer: D) -> Result<Vec<RawParameterSyntax>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawParameterSyntax, MAX_PARAMETERS_PER_FUNCTION>(
        deserializer,
        "parameters",
    )
}

fn deserialize_statements<'de, D>(deserializer: D) -> Result<Vec<RawStatementSyntax>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawStatementSyntax, MAX_STATEMENTS_PER_FUNCTION>(
        deserializer,
        "statements",
    )
}

fn deserialize_expressions<'de, D>(deserializer: D) -> Result<Vec<RawExpressionSyntax>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawExpressionSyntax, MAX_EXPRESSIONS_PER_FUNCTION>(
        deserializer,
        "expressions",
    )
}

fn deserialize_diagnostics<'de, D>(deserializer: D) -> Result<Vec<RawProviderDiagnostic>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded::<D, RawProviderDiagnostic, MAX_PROVIDER_DIAGNOSTICS>(
        deserializer,
        "provider diagnostics",
    )
}

fn deserialize_bounded<'de, D, T, const MAX: usize>(
    deserializer: D,
    label: &'static str,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct BoundedVisitor<T, const MAX: usize> {
        label: &'static str,
        marker: PhantomData<T>,
    }

    impl<'de, T, const MAX: usize> Visitor<'de> for BoundedVisitor<T, MAX>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "at most {MAX} {}", self.label)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let capacity = sequence.size_hint().unwrap_or(0).min(MAX);
            let mut values = Vec::new();
            values.try_reserve(capacity).map_err(|_| {
                A::Error::custom(format_args!("unable to reserve bounded {} storage", self.label))
            })?;
            while values.len() < MAX {
                let Some(value) = sequence.next_element()? else {
                    return Ok(values);
                };
                values.push(value);
            }
            if sequence.next_element::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom(format_args!(
                    "{} exceed the limit of {MAX}",
                    self.label
                )));
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(BoundedVisitor::<T, MAX> { label, marker: PhantomData })
}

/// Decodes a strict protocol-v2 response after applying the transport byte bound.
///
/// # Errors
///
/// Returns a stable failure for oversized bytes or any malformed, incomplete, or unknown field.
pub fn decode_snapshot(bytes: &[u8]) -> Result<RawProjectSyntaxSnapshot, SyntaxDecodeError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(SyntaxDecodeError::ResponseTooLarge {
            actual: bytes.len(),
            limit: MAX_RESPONSE_BYTES,
        });
    }
    serde_json::from_slice(bytes).map_err(|_| SyntaxDecodeError::InvalidSnapshot)
}

/// Verifies the complete hostile provider response against one authoritative source map.
///
/// # Errors
///
/// Returns deterministic bounded diagnostics for every rejected protocol invariant.
pub fn verify_snapshot(
    mut raw: RawProjectSyntaxSnapshot,
    sources: &SourceMap,
) -> Result<ProjectSyntaxSnapshot, Vec<Diagnostic>> {
    if let Some(error) = snapshot_budget_error(&raw) {
        return Err(vec![error]);
    }
    let mut errors = ValidationErrors::default();
    if raw.schema_version != PROTOCOL_VERSION {
        errors.push(protocol_error(
            None,
            format!(
                "syntax snapshot uses schema version {}; expected {PROTOCOL_VERSION}",
                raw.schema_version
            ),
            "return the exact negotiated protocol-v2 schema version",
        ));
    }
    if raw.files.len() != sources.len() {
        errors.push(protocol_error(
            None,
            format!(
                "syntax snapshot contains {} files; the authoritative source map contains {}",
                raw.files.len(),
                sources.len()
            ),
            "return every requested source file exactly once and no additional files",
        ));
    }
    raw.files.sort();
    let mut seen = BTreeSet::new();
    let files = raw
        .files
        .into_iter()
        .filter_map(|file| verify_file(file, sources, &mut seen, &mut errors))
        .collect();
    let diagnostics = verify_provider_diagnostics(raw.diagnostics, sources, &mut errors);
    if errors.is_empty() {
        Ok(ProjectSyntaxSnapshot { schema_version: PROTOCOL_VERSION, files, diagnostics })
    } else {
        Err(errors.finish())
    }
}

fn snapshot_budget_error(raw: &RawProjectSyntaxSnapshot) -> Option<Diagnostic> {
    if raw.files.len() > MAX_SOURCE_FILES {
        return Some(limit_error(format!(
            "syntax snapshot contains {} files; the limit is {MAX_SOURCE_FILES}",
            raw.files.len()
        )));
    }
    if raw.diagnostics.len() > MAX_PROVIDER_DIAGNOSTICS {
        return Some(limit_error(format!(
            "syntax snapshot contains {} provider diagnostics; the limit is {MAX_PROVIDER_DIAGNOSTICS}",
            raw.diagnostics.len()
        )));
    }
    if raw.files.iter().any(|file| file.functions.len() > MAX_FUNCTIONS_PER_FILE) {
        return Some(limit_error("one source unit exceeds the per-file function limit"));
    }
    if raw.files.iter().flat_map(|file| &file.functions).any(|function| {
        function.parameters.len() > MAX_PARAMETERS_PER_FUNCTION
            || function.body.statements.len() > MAX_STATEMENTS_PER_FUNCTION
            || function.body.expressions.len() > MAX_EXPRESSIONS_PER_FUNCTION
    }) {
        return Some(limit_error("one function exceeds a protocol-v2 collection limit"));
    }
    let mut functions = 0_usize;
    let mut parameters = 0_usize;
    let mut statements = 0_usize;
    let mut expressions = 0_usize;
    for function in raw.files.iter().flat_map(|file| &file.functions) {
        let Some(next_functions) = functions.checked_add(1) else {
            return Some(limit_error("syntax function count overflowed"));
        };
        let Some(next_parameters) = parameters.checked_add(function.parameters.len()) else {
            return Some(limit_error("syntax parameter count overflowed"));
        };
        let Some(next_statements) = statements.checked_add(function.body.statements.len()) else {
            return Some(limit_error("syntax statement count overflowed"));
        };
        let Some(next_expressions) = expressions.checked_add(function.body.expressions.len())
        else {
            return Some(limit_error("syntax expression count overflowed"));
        };
        functions = next_functions;
        parameters = next_parameters;
        statements = next_statements;
        expressions = next_expressions;
    }
    if functions > MAX_FUNCTIONS_PER_PROJECT
        || parameters > MAX_PARAMETERS_PER_PROJECT
        || statements > MAX_STATEMENTS_PER_PROJECT
        || expressions > MAX_EXPRESSIONS_PER_PROJECT
    {
        return Some(limit_error("syntax snapshot exceeds an aggregate collection limit"));
    }
    None
}

fn verify_file(
    mut raw: RawSourceUnit,
    sources: &SourceMap,
    seen: &mut BTreeSet<u32>,
    errors: &mut ValidationErrors,
) -> Option<SourceUnit> {
    if !seen.insert(raw.id) {
        errors.push(protocol_error(
            None,
            format!("syntax snapshot repeats file identifier {}", raw.id),
            "return each authoritative file identifier exactly once",
        ));
        return None;
    }
    let Ok(id) = sources.verify_file_id(raw.id) else {
        errors.push(protocol_error(
            None,
            format!("syntax snapshot contains unknown file identifier {}", raw.id),
            "return only identifiers issued by the authoritative source map",
        ));
        return None;
    };
    let Ok(path) = NormalizedSourcePath::new(raw.path) else {
        errors.push(protocol_error(
            None,
            format!("syntax snapshot contains an unsafe path for file {}", raw.id),
            "return the exact normalized path from the source request",
        ));
        return None;
    };
    if sources.source(id).is_none_or(|source| source.path() != &path) {
        errors.push(protocol_error(
            Some(path.as_str().to_owned()),
            format!("syntax path disagrees with authoritative file identifier {}", raw.id),
            "return the exact identifier and path pair from the source request",
        ));
        return None;
    }
    raw.functions.sort();
    let mut functions = Vec::with_capacity(raw.functions.len());
    let mut previous_end = None;
    for (index, function) in raw.functions.into_iter().enumerate() {
        if let Some(function) = verify_function(function, index, raw.id, &path, sources, errors) {
            if previous_end.is_some_and(|end| function.span.start() < end) {
                errors.push(node_error(
                    Some(path.as_str().to_owned()),
                    format!("exported function {index} overlaps the preceding function"),
                    "return source-ordered non-overlapping top-level declarations",
                ));
            }
            previous_end = Some(function.span.end());
            functions.push(function);
        }
    }
    Some(SourceUnit { id, path, functions })
}

fn verify_function(
    raw: RawFunctionSyntax,
    index: usize,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
) -> Option<FunctionSyntax> {
    let span = verify_node_span(raw.span, file, path, sources, errors, "function")?;
    let export_span = verify_node_span(raw.export_span, file, path, sources, errors, "export")?;
    require_source_text(export_span, "export", path, sources, errors, "export keyword");
    let function_span =
        verify_node_span(raw.function_span, file, path, sources, errors, "function keyword")?;
    require_source_text(function_span, "function", path, sources, errors, "function keyword");
    let name = verify_identifier(raw.name, file, path, sources, errors, "function name")?;
    let result_type = verify_type(raw.result_type, file, path, sources, errors, "result type")?;
    let body_span = verify_node_span(raw.body.span, file, path, sources, errors, "function body")?;
    for (label, child) in [
        ("export keyword", export_span),
        ("function keyword", function_span),
        ("function name", name.span),
        ("result type", result_type.span),
        ("function body", body_span),
    ] {
        require_contains(span, child, path, errors, "function", label);
    }
    require_before(export_span, function_span, path, errors, "export keyword", "function keyword");
    require_before(function_span, name.span, path, errors, "function keyword", "function name");
    require_before(result_type.span, body_span, path, errors, "result type", "function body");

    let mut parameters = Vec::with_capacity(raw.parameters.len());
    let mut previous_end = None;
    for (parameter_index, raw_parameter) in raw.parameters.into_iter().enumerate() {
        let parameter =
            verify_parameter(raw_parameter, file, path, sources, errors, parameter_index)?;
        require_contains(span, parameter.span, path, errors, "function", "parameter");
        if previous_end.is_some_and(|end| parameter.span.start() < end) {
            errors.push(node_error(
                Some(path.as_str().to_owned()),
                format!("function {index} has overlapping or out-of-order parameters"),
                "return parameters in non-overlapping source order",
            ));
        }
        previous_end = Some(parameter.span.end());
        parameters.push(parameter);
    }
    if let Some(first) = parameters.first() {
        require_before(name.span, first.span, path, errors, "function name", "first parameter");
    }
    if let Some(last) = parameters.last() {
        require_before(last.span, result_type.span, path, errors, "last parameter", "result type");
    } else {
        require_before(name.span, result_type.span, path, errors, "function name", "result type");
    }

    let body = verify_body(raw.body, body_span, file, path, sources, errors)?;
    Some(FunctionSyntax { span, export_span, function_span, name, parameters, result_type, body })
}

fn verify_parameter(
    raw: RawParameterSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
    index: usize,
) -> Option<ParameterSyntax> {
    let span = verify_node_span(raw.span, file, path, sources, errors, "parameter")?;
    let name = verify_identifier(raw.name, file, path, sources, errors, "parameter name")?;
    let type_syntax = verify_type(raw.type_syntax, file, path, sources, errors, "parameter type")?;
    require_contains(span, name.span, path, errors, "parameter", "name");
    require_contains(span, type_syntax.span, path, errors, "parameter", "type");
    require_before(name.span, type_syntax.span, path, errors, "parameter name", "parameter type");
    if span.start() == span.end() {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("parameter {index} has an empty full span"),
            "return the complete parameter source range",
        ));
    }
    Some(ParameterSyntax { span, name, type_syntax })
}

fn verify_type(
    raw: RawTypeSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
    label: &str,
) -> Option<TypeSyntax> {
    let span = verify_node_span(raw.span, file, path, sources, errors, label)?;
    let kind = match raw.kind {
        RawTypeSyntaxKind::Missing => {
            if span.start() != span.end() {
                errors.push(node_error(
                    Some(path.as_str().to_owned()),
                    format!("{label} marks a missing annotation with a non-empty span"),
                    "use one empty insertion-point span for a missing type annotation",
                ));
            }
            TypeSyntaxKind::Missing
        }
        RawTypeSyntaxKind::Named { name } => {
            if !bounded_text(&name, MAX_NAME_CHARACTERS) {
                errors.push(limit_error(format!("{label} exceeds the identifier length limit")));
                return None;
            }
            require_source_text(span, &name, path, sources, errors, label);
            TypeSyntaxKind::Named { name }
        }
    };
    Some(TypeSyntax { span, kind })
}

fn verify_identifier(
    raw: RawIdentifierSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
    label: &str,
) -> Option<IdentifierSyntax> {
    if !bounded_text(&raw.text, MAX_NAME_CHARACTERS) {
        errors.push(limit_error(format!("{label} exceeds the identifier length limit")));
        return None;
    }
    let span = verify_node_span(raw.span, file, path, sources, errors, label)?;
    require_source_text(span, &raw.text, path, sources, errors, label);
    Some(IdentifierSyntax { text: raw.text, span })
}

fn verify_body(
    raw: RawFunctionBodySyntax,
    span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
) -> Option<FunctionBodySyntax> {
    let (expressions, mut owners) =
        verify_expressions(raw.expressions, span, file, path, sources, errors)?;
    let mut statements = Vec::with_capacity(raw.statements.len());
    let mut previous_end = None;
    for (index, raw_statement) in raw.statements.into_iter().enumerate() {
        let statement = verify_statement(
            &raw_statement,
            index,
            span,
            file,
            path,
            sources,
            &expressions,
            &mut owners,
            errors,
        )?;
        if previous_end.is_some_and(|end| statement.span.start() < end) {
            errors.push(node_error(
                Some(path.as_str().to_owned()),
                "function body has overlapping or out-of-order statements",
                "return statements in non-overlapping source order",
            ));
        }
        previous_end = Some(statement.span.end());
        statements.push(statement);
    }
    verify_canonical_postorder(&statements, &expressions, path, errors);
    for (index, owners) in owners.into_iter().enumerate() {
        if owners != 1 {
            errors.push(node_error(
                Some(path.as_str().to_owned()),
                format!("expression {index} has {owners} owners; expected exactly one"),
                "return one source tree with no shared or orphan arena entries",
            ));
        }
    }
    Some(FunctionBodySyntax { span, statements, expressions })
}

fn verify_canonical_postorder(
    statements: &[StatementSyntax],
    expressions: &[ExpressionSyntax],
    path: &NormalizedSourcePath,
    errors: &mut ValidationErrors,
) {
    let mut emitted = vec![false; expressions.len()];
    let mut expected = 0_usize;
    for statement in statements {
        let StatementKind::Return { value, .. } = statement.kind;
        let Ok(root) = usize::try_from(value.index()) else {
            continue;
        };
        let mut stack = vec![(root, false)];
        while let Some((index, exiting)) = stack.pop() {
            let Some(expression) = expressions.get(index) else {
                continue;
            };
            if exiting {
                if emitted[index] {
                    continue;
                }
                if index != expected {
                    errors.push(node_error(
                        Some(path.as_str().to_owned()),
                        format!(
                            "expression arena is not canonical postorder: expected {expected}, found {index}"
                        ),
                        "emit each statement tree left-to-right in exact postorder",
                    ));
                }
                emitted[index] = true;
                expected = expected.saturating_add(1);
                continue;
            }
            if emitted[index] {
                continue;
            }
            stack.push((index, true));
            if let ExpressionKind::Addition { lhs, rhs, .. } = expression.kind {
                if let Ok(rhs) = usize::try_from(rhs.index()) {
                    stack.push((rhs, false));
                }
                if let Ok(lhs) = usize::try_from(lhs.index()) {
                    stack.push((lhs, false));
                }
            }
        }
    }
}

fn verify_expressions(
    raw: Vec<RawExpressionSyntax>,
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
) -> Option<(Vec<ExpressionSyntax>, Vec<u32>)> {
    let mut expressions: Vec<ExpressionSyntax> = Vec::with_capacity(raw.len());
    let mut depths: Vec<u32> = Vec::with_capacity(raw.len());
    let mut owners = vec![0_u32; raw.len()];
    for (index, raw_expression) in raw.into_iter().enumerate() {
        let span =
            verify_node_span(raw_expression.span, file, path, sources, errors, "expression")?;
        require_contains(body_span, span, path, errors, "function body", "expression");
        let (kind, depth) = match raw_expression.kind {
            RawExpressionKind::Reference { name } => {
                let name = verify_identifier(name, file, path, sources, errors, "reference name")?;
                require_contains(span, name.span, path, errors, "reference", "name");
                (ExpressionKind::Reference { name }, 1)
            }
            RawExpressionKind::BoolLiteral { value } => {
                let spelling = if value { "true" } else { "false" };
                require_source_text(span, spelling, path, sources, errors, "Boolean literal");
                (ExpressionKind::BoolLiteral { value }, 1)
            }
            RawExpressionKind::I32Literal { spelling } => {
                verify_i32_literal(spelling, index, span, path, sources, errors)?
            }
            RawExpressionKind::Addition { operator_span, lhs, rhs } => {
                let operator_span = verify_node_span(
                    operator_span,
                    file,
                    path,
                    sources,
                    errors,
                    "addition operator",
                )?;
                require_source_text(operator_span, "+", path, sources, errors, "addition operator");
                require_contains(span, operator_span, path, errors, "addition", "operator");
                let Ok(lhs_index) = usize::try_from(lhs) else {
                    errors.push(invalid_arena_edge(path, index));
                    return None;
                };
                let Ok(rhs_index) = usize::try_from(rhs) else {
                    errors.push(invalid_arena_edge(path, index));
                    return None;
                };
                if lhs == rhs || lhs_index >= index || rhs_index >= index {
                    errors.push(invalid_arena_edge(path, index));
                    return None;
                }
                require_contains(
                    span,
                    expressions[lhs_index].span,
                    path,
                    errors,
                    "addition",
                    "left operand",
                );
                require_contains(
                    span,
                    expressions[rhs_index].span,
                    path,
                    errors,
                    "addition",
                    "right operand",
                );
                require_before(
                    expressions[lhs_index].span,
                    operator_span,
                    path,
                    errors,
                    "left operand",
                    "addition operator",
                );
                require_before(
                    operator_span,
                    expressions[rhs_index].span,
                    path,
                    errors,
                    "addition operator",
                    "right operand",
                );
                owners[lhs_index] = owners[lhs_index].saturating_add(1);
                owners[rhs_index] = owners[rhs_index].saturating_add(1);
                let depth = depths[lhs_index].max(depths[rhs_index]).saturating_add(1);
                (
                    ExpressionKind::Addition {
                        operator_span,
                        lhs: ExpressionId(lhs),
                        rhs: ExpressionId(rhs),
                    },
                    depth,
                )
            }
        };
        if depth > MAX_EXPRESSION_DEPTH {
            errors.push(limit_error(format!(
                "expression {index} has depth {depth}; the limit is {MAX_EXPRESSION_DEPTH}"
            )));
            return None;
        }
        depths.push(depth);
        expressions.push(ExpressionSyntax { span, kind });
    }
    Some((expressions, owners))
}

fn verify_i32_literal(
    spelling: String,
    index: usize,
    span: Span,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
) -> Option<(ExpressionKind, u32)> {
    if !valid_integer_spelling(&spelling) {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("expression {index} has an invalid decimal i32 spelling"),
            "return a bounded canonical base-10 integer spelling",
        ));
        return None;
    }
    require_source_text(span, &spelling, path, sources, errors, "i32 literal");
    Some((ExpressionKind::I32Literal { spelling }, 1))
}

#[allow(clippy::too_many_arguments)]
fn verify_statement(
    raw: &RawStatementSyntax,
    index: usize,
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    expressions: &[ExpressionSyntax],
    owners: &mut [u32],
    errors: &mut ValidationErrors,
) -> Option<StatementSyntax> {
    let span = verify_node_span(raw.span, file, path, sources, errors, "statement")?;
    require_contains(body_span, span, path, errors, "function body", "statement");
    let kind = match &raw.kind {
        RawStatementKind::Return { keyword_span, value } => {
            let keyword_span =
                verify_node_span(*keyword_span, file, path, sources, errors, "return keyword")?;
            require_source_text(keyword_span, "return", path, sources, errors, "return keyword");
            require_contains(span, keyword_span, path, errors, "return statement", "keyword");
            let Ok(value_index) = usize::try_from(*value) else {
                errors.push(invalid_return_root(path, index));
                return None;
            };
            let Some(expression) = expressions.get(value_index) else {
                errors.push(invalid_return_root(path, index));
                return None;
            };
            require_contains(span, expression.span, path, errors, "return statement", "value");
            require_before(
                keyword_span,
                expression.span,
                path,
                errors,
                "return keyword",
                "return value",
            );
            owners[value_index] = owners[value_index].saturating_add(1);
            StatementKind::Return { keyword_span, value: ExpressionId(*value) }
        }
    };
    Some(StatementSyntax { span, kind })
}

fn verify_provider_diagnostics(
    mut raw: Vec<RawProviderDiagnostic>,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
) -> Vec<Diagnostic> {
    raw.sort();
    raw.into_iter()
        .filter_map(|raw| {
            if !bounded_text(&raw.code, MAX_NAME_CHARACTERS)
                || raw.message.chars().count() > MAX_DIAGNOSTIC_TEXT_CHARACTERS
                || raw.guidance.chars().count() > MAX_DIAGNOSTIC_TEXT_CHARACTERS
            {
                errors.push(limit_error("provider diagnostic exceeds a text limit"));
                return None;
            }
            let span = match raw.location {
                RawDiagnosticLocation::Global => None,
                RawDiagnosticLocation::Source { span } => {
                    let Ok(span) = sources.verify_span(span) else {
                        errors.push(node_error(
                            None,
                            "provider diagnostic contains an invalid source span",
                            "return only source ranges issued by the authoritative source map",
                        ));
                        return None;
                    };
                    Some(span)
                }
            };
            Some(match (raw.severity, span) {
                (Severity::Error, Some(span)) => {
                    Diagnostic::error_at(raw.code, span, raw.message, raw.guidance)
                }
                (Severity::Warning, Some(span)) => {
                    Diagnostic::warning_at(raw.code, span, raw.message, raw.guidance)
                }
                (Severity::Error, None) => {
                    Diagnostic::error(raw.code, None, raw.message, raw.guidance)
                }
                (Severity::Warning, None) => {
                    Diagnostic::warning(raw.code, None, raw.message, raw.guidance)
                }
            })
        })
        .collect()
}

fn verify_node_span(
    raw: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
    label: &str,
) -> Option<Span> {
    if raw.file != file {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{label} references file {} instead of containing file {file}", raw.file),
            "keep every syntax-node span inside its containing source unit",
        ));
        return None;
    }
    let Ok(span) = sources.verify_span(raw) else {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{label} contains an invalid UTF-8 source span"),
            "return ordered in-range UTF-8 boundaries from the authoritative source text",
        ));
        return None;
    };
    Some(span)
}

fn require_source_text(
    span: Span,
    expected: &str,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut ValidationErrors,
    label: &str,
) {
    let Ok(resolved) = sources.resolve(span) else {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{label} cannot be resolved through the authoritative source map"),
            "return a source range issued by the authoritative source map",
        ));
        return;
    };
    let (Ok(start), Ok(end)) =
        (usize::try_from(resolved.start.byte_offset), usize::try_from(resolved.end.byte_offset))
    else {
        errors.push(limit_error(format!("{label} source offset exceeds the host index range")));
        return;
    };
    if resolved.source().text().get(start..end) != Some(expected) {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{label} spelling disagrees with the authoritative source text"),
            "return the exact source spelling and range without provider substitution",
        ));
    }
}

fn require_contains(
    parent: Span,
    child: Span,
    path: &NormalizedSourcePath,
    errors: &mut ValidationErrors,
    parent_label: &str,
    child_label: &str,
) {
    if parent.file() != child.file() || child.start() < parent.start() || child.end() > parent.end()
    {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{child_label} span is outside its {parent_label} span"),
            "return structurally nested source ranges for every syntax node",
        ));
    }
}

fn require_before(
    left: Span,
    right: Span,
    path: &NormalizedSourcePath,
    errors: &mut ValidationErrors,
    left_label: &str,
    right_label: &str,
) {
    if left.file() != right.file() || left.end() > right.start() {
        errors.push(node_error(
            Some(path.as_str().to_owned()),
            format!("{left_label} is not before {right_label} in source order"),
            "return canonical source-ordered spans for protocol-v2 syntax nodes",
        ));
    }
}

fn bounded_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.chars().count() <= limit && !value.chars().any(char::is_control)
}

fn valid_integer_spelling(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_LITERAL_BYTES || !value.is_ascii() {
        return false;
    }
    if value == "0" {
        return true;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn invalid_arena_edge(path: &NormalizedSourcePath, index: usize) -> Diagnostic {
    node_error(
        Some(path.as_str().to_owned()),
        format!("addition expression {index} has a self, shared, or non-postorder child edge"),
        "emit distinct child nodes before each parent in canonical postorder",
    )
}

fn invalid_return_root(path: &NormalizedSourcePath, index: usize) -> Diagnostic {
    node_error(
        Some(path.as_str().to_owned()),
        format!("return statement {index} references an unknown expression"),
        "reference an in-range expression arena entry",
    )
}

fn protocol_error(
    path: Option<String>,
    message: impl Into<String>,
    guidance: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error("ZRYNA-Y1001", path, message, guidance)
}

fn node_error(
    path: Option<String>,
    message: impl Into<String>,
    guidance: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error("ZRYNA-Y1002", path, message, guidance)
}

fn limit_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-Y1003",
        None,
        message,
        "reduce the provider response to the documented protocol-v2 limits",
    )
}

#[derive(Default)]
struct ValidationErrors {
    diagnostics: Vec<Diagnostic>,
    truncated: bool,
}

impl ValidationErrors {
    fn push(&mut self, diagnostic: Diagnostic) {
        let retained_limit = MAX_VALIDATION_ERRORS.saturating_sub(1);
        if self.diagnostics.len() < retained_limit {
            self.diagnostics.push(diagnostic);
            return;
        }
        self.truncated = true;
        let worst = self
            .diagnostics
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| compare_diagnostics(left, right))
            .map(|(index, _)| index);
        if let Some(index) = worst
            && compare_diagnostics(&diagnostic, &self.diagnostics[index]).is_lt()
        {
            self.diagnostics[index] = diagnostic;
        }
    }

    fn is_empty(&self) -> bool {
        self.diagnostics.is_empty() && !self.truncated
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort_by(compare_diagnostics);
        if self.truncated {
            self.diagnostics.push(limit_error(
                "syntax validation stopped at the deterministic diagnostic limit",
            ));
        }
        self.diagnostics
    }
}

fn diagnostic_key(
    diagnostic: &Diagnostic,
) -> (u8, &str, u32, u32, u32, Severity, &str, &str, &str) {
    let (location_kind, path, file, start, end) = match diagnostic.primary() {
        PrimaryLocation::Global => (0, "", 0, 0, 0),
        PrimaryLocation::WorkspacePath { path } => (1, path.as_str(), 0, 0, 0),
        PrimaryLocation::Source { span } => (2, "", span.file().index(), span.start(), span.end()),
    };
    (
        location_kind,
        path,
        file,
        start,
        end,
        diagnostic.severity(),
        diagnostic.code(),
        diagnostic.message(),
        diagnostic.guidance(),
    )
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> std::cmp::Ordering {
    diagnostic_key(left).cmp(&diagnostic_key(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_source::SourceFileInput;

    const SOURCE: &str = "export function add(a: i32, b: i32): i32 { return a + b; }";

    fn raw_span(file: u32, start: usize, end: usize) -> UntrustedSpan {
        UntrustedSpan {
            file,
            start: u32::try_from(start).expect("fixture start must fit"),
            end: u32::try_from(end).expect("fixture end must fit"),
        }
    }

    fn range_from(text: &str, needle: &str, from: usize) -> (usize, usize) {
        let start = from
            .checked_add(text[from..].find(needle).expect("fixture token must exist"))
            .expect("fixture offset must fit");
        (start, start + needle.len())
    }

    fn sources(text: &str) -> SourceMap {
        SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: text.to_owned(),
        }])
        .expect("fixture source map must be valid")
    }

    fn raw_parameter(
        span: (usize, usize),
        name_span: (usize, usize),
        type_span: (usize, usize),
        name: &str,
    ) -> RawParameterSyntax {
        RawParameterSyntax {
            span: raw_span(0, span.0, span.1),
            name: RawIdentifierSyntax {
                text: name.to_owned(),
                span: raw_span(0, name_span.0, name_span.1),
            },
            type_syntax: RawTypeSyntax {
                span: raw_span(0, type_span.0, type_span.1),
                kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
            },
        }
    }

    fn valid_raw(text: &str) -> RawProjectSyntaxSnapshot {
        let (export_start, export_end) = range_from(text, "export", 0);
        let (function_start, function_end) = range_from(text, "function", export_end);
        let (name_start, name_end) = range_from(text, "add", function_end);
        let (first_parameter_start, first_parameter_end) = range_from(text, "a: i32", name_end);
        let (first_name_start, first_name_end) = range_from(text, "a", first_parameter_start);
        let (first_type_start, first_type_end) = range_from(text, "i32", first_name_end);
        let (second_parameter_start, second_parameter_end) =
            range_from(text, "b: i32", first_parameter_end);
        let (second_name_start, second_name_end) = range_from(text, "b", second_parameter_start);
        let (second_type_start, second_type_end) = range_from(text, "i32", second_name_end);
        let (result_start, result_end) = range_from(text, "i32", second_parameter_end);
        let (body_start, _) = range_from(text, "{", result_end);
        let (return_start, return_keyword_end) = range_from(text, "return", body_start);
        let (addition_start, addition_end) = range_from(text, "a + b", return_keyword_end);
        let (lhs_start, lhs_end) = range_from(text, "a", addition_start);
        let (operator_start, operator_end) = range_from(text, "+", lhs_end);
        let (rhs_start, rhs_end) = range_from(text, "b", operator_end);
        let (semicolon_start, semicolon_end) = range_from(text, ";", addition_end);
        let _ = semicolon_start;
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                functions: vec![RawFunctionSyntax {
                    span: raw_span(0, export_start, text.len()),
                    export_span: raw_span(0, export_start, export_end),
                    function_span: raw_span(0, function_start, function_end),
                    name: RawIdentifierSyntax {
                        text: "add".to_owned(),
                        span: raw_span(0, name_start, name_end),
                    },
                    parameters: vec![
                        raw_parameter(
                            (first_parameter_start, first_parameter_end),
                            (first_name_start, first_name_end),
                            (first_type_start, first_type_end),
                            "a",
                        ),
                        raw_parameter(
                            (second_parameter_start, second_parameter_end),
                            (second_name_start, second_name_end),
                            (second_type_start, second_type_end),
                            "b",
                        ),
                    ],
                    result_type: RawTypeSyntax {
                        span: raw_span(0, result_start, result_end),
                        kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                    },
                    body: RawFunctionBodySyntax {
                        span: raw_span(0, body_start, text.len()),
                        statements: vec![RawStatementSyntax {
                            span: raw_span(0, return_start, semicolon_end),
                            kind: RawStatementKind::Return {
                                keyword_span: raw_span(0, return_start, return_keyword_end),
                                value: 2,
                            },
                        }],
                        expressions: vec![
                            RawExpressionSyntax {
                                span: raw_span(0, lhs_start, lhs_end),
                                kind: RawExpressionKind::Reference {
                                    name: RawIdentifierSyntax {
                                        text: "a".to_owned(),
                                        span: raw_span(0, lhs_start, lhs_end),
                                    },
                                },
                            },
                            RawExpressionSyntax {
                                span: raw_span(0, rhs_start, rhs_end),
                                kind: RawExpressionKind::Reference {
                                    name: RawIdentifierSyntax {
                                        text: "b".to_owned(),
                                        span: raw_span(0, rhs_start, rhs_end),
                                    },
                                },
                            },
                            RawExpressionSyntax {
                                span: raw_span(0, addition_start, addition_end),
                                kind: RawExpressionKind::Addition {
                                    operator_span: raw_span(0, operator_start, operator_end),
                                    lhs: 0,
                                    rhs: 1,
                                },
                            },
                        ],
                    },
                }],
            }],
            diagnostics: vec![RawProviderDiagnostic {
                code: "TS9000".to_owned(),
                severity: Severity::Warning,
                location: RawDiagnosticLocation::Source { span: raw_span(0, name_start, name_end) },
                message: "provider note".to_owned(),
                guidance: "review the declaration".to_owned(),
            }],
        }
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code() == code)
    }

    #[test]
    fn valid_snapshot_round_trips_and_exposes_only_verified_nodes() {
        let sources = sources(SOURCE);
        let bytes = serde_json::to_vec(&valid_raw(SOURCE)).expect("fixture must serialize");
        let raw = decode_snapshot(&bytes).expect("valid wire snapshot must decode");
        let project = verify_snapshot(raw, &sources).expect("valid snapshot must verify");

        assert_eq!(project.schema_version(), PROTOCOL_VERSION);
        assert_eq!(project.files().len(), 1);
        assert_eq!(project.files()[0].path().as_str(), "src/main.zry");
        let function = &project.files()[0].functions()[0];
        assert_eq!(function.name().text(), "add");
        assert_eq!(function.parameters().len(), 2);
        assert_eq!(function.body().expressions().len(), 3);
        assert_eq!(function.body().statements().len(), 1);
        assert_eq!(project.diagnostics().len(), 1);
        for expression in function.body().expressions() {
            assert!(sources.resolve(expression.span()).is_ok());
        }
        assert!(project.is_bound_to(&sources));
        assert!(!project.is_bound_to(&self::sources(SOURCE)));
    }

    #[test]
    fn decoder_rejects_unknown_missing_duplicate_trailing_and_oversized_input() {
        for bytes in [
            br#"{"schema_version":2,"files":[],"diagnostics":[],"unknown":true}"#.as_slice(),
            br#"{"schema_version":2,"files":[]}"#.as_slice(),
            br#"{"schema_version":2,"schema_version":2,"files":[],"diagnostics":[]}"#.as_slice(),
            br#"{"schema_version":2,"files":[],"diagnostics":[]} true"#.as_slice(),
        ] {
            assert_eq!(
                decode_snapshot(bytes).expect_err("invalid wire shape must fail").code(),
                "ZRYNA-Y1001"
            );
        }
        let oversized = vec![b' '; MAX_RESPONSE_BYTES + 1];
        assert_eq!(
            decode_snapshot(&oversized).expect_err("oversized response must fail").code(),
            "ZRYNA-Y1003"
        );
    }

    #[test]
    fn bounded_sequence_deserialization_rejects_the_first_extra_item() {
        let mut raw = valid_raw(SOURCE);
        let parameter = raw.files[0].functions[0].parameters[0].clone();
        raw.files[0].functions[0].parameters = vec![parameter; MAX_PARAMETERS_PER_FUNCTION + 1];
        let bytes = serde_json::to_vec(&raw).expect("programmatic hostile value must serialize");
        assert!(decode_snapshot(&bytes).is_err());
    }

    #[test]
    fn exact_file_identity_and_utf8_boundaries_fail_closed() {
        let unicode_source = format!("😀{SOURCE}");
        let sources = sources(&unicode_source);
        let mut raw = valid_raw(&unicode_source);
        raw.files[0].functions[0].export_span = raw_span(0, 1, 2);
        let diagnostics =
            verify_snapshot(raw, &sources).expect_err("mid-code-point span must fail");
        assert!(has_code(&diagnostics, "ZRYNA-Y1002"));

        let mut raw = valid_raw(&unicode_source);
        raw.files[0].path = "SRC/MAIN.ZRY".to_owned();
        let diagnostics = verify_snapshot(raw, &sources).expect_err("case-variant path must fail");
        assert!(has_code(&diagnostics, "ZRYNA-Y1001"));
    }

    #[test]
    fn provider_values_must_match_authoritative_source_spelling() {
        let sources = sources(SOURCE);
        let mut hostile_values = Vec::new();

        let mut name = valid_raw(SOURCE);
        name.files[0].functions[0].name.text = "substituted".to_owned();
        hostile_values.push(name);

        let mut type_name = valid_raw(SOURCE);
        type_name.files[0].functions[0].result_type.kind =
            RawTypeSyntaxKind::Named { name: "bool".to_owned() };
        hostile_values.push(type_name);

        let mut literal = valid_raw(SOURCE);
        literal.files[0].functions[0].body.expressions[0].kind =
            RawExpressionKind::I32Literal { spelling: "1".to_owned() };
        hostile_values.push(literal);

        let mut boolean = valid_raw(SOURCE);
        boolean.files[0].functions[0].body.expressions[1].kind =
            RawExpressionKind::BoolLiteral { value: true };
        hostile_values.push(boolean);

        let mut export_keyword = valid_raw(SOURCE);
        export_keyword.files[0].functions[0].export_span =
            export_keyword.files[0].functions[0].name.span;
        hostile_values.push(export_keyword);

        let mut return_keyword = valid_raw(SOURCE);
        let expression_span = return_keyword.files[0].functions[0].body.expressions[0].span;
        let RawStatementKind::Return { keyword_span, .. } =
            &mut return_keyword.files[0].functions[0].body.statements[0].kind;
        *keyword_span = expression_span;
        hostile_values.push(return_keyword);

        let mut function_keyword = valid_raw(SOURCE);
        function_keyword.files[0].functions[0].function_span =
            function_keyword.files[0].functions[0].name.span;
        hostile_values.push(function_keyword);

        let mut operator = valid_raw(SOURCE);
        let lhs_span = operator.files[0].functions[0].body.expressions[0].span;
        let RawExpressionKind::Addition { operator_span, .. } =
            &mut operator.files[0].functions[0].body.expressions[2].kind
        else {
            unreachable!("fixture root must be an addition");
        };
        *operator_span = lhs_span;
        hostile_values.push(operator);

        for raw in hostile_values {
            let diagnostics =
                verify_snapshot(raw, &sources).expect_err("substituted provider text must fail");
            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.message().contains("spelling disagrees with the authoritative source")
            }));
        }

        assert!(!valid_integer_spelling("-0"));
        assert!(!valid_integer_spelling("01"));
        assert!(valid_integer_spelling("-1"));
    }

    #[test]
    fn arena_rejects_self_edges_shared_nodes_and_orphans() {
        let sources = sources(SOURCE);
        let mut self_edge = valid_raw(SOURCE);
        let RawExpressionKind::Addition { operator_span, .. } =
            self_edge.files[0].functions[0].body.expressions[2].kind
        else {
            unreachable!("fixture root must be an addition");
        };
        self_edge.files[0].functions[0].body.expressions[2].kind =
            RawExpressionKind::Addition { operator_span, lhs: 2, rhs: 1 };
        let diagnostics = verify_snapshot(self_edge, &sources).expect_err("self edge must fail");
        assert!(has_code(&diagnostics, "ZRYNA-Y1002"));

        let mut orphan = valid_raw(SOURCE);
        let orphan_span = orphan.files[0].functions[0].body.expressions[0].span;
        orphan.files[0].functions[0].body.expressions.push(RawExpressionSyntax {
            span: orphan_span,
            kind: RawExpressionKind::BoolLiteral { value: true },
        });
        let diagnostics = verify_snapshot(orphan, &sources).expect_err("orphan node must fail");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.message().contains("0 owners")));

        let mut shared = valid_raw(SOURCE);
        let mut repeated_root = shared.files[0].functions[0].body.statements[0].clone();
        let RawStatementKind::Return { value, .. } = &mut repeated_root.kind;
        *value = 0;
        shared.files[0].functions[0].body.statements.push(repeated_root);
        let diagnostics = verify_snapshot(shared, &sources).expect_err("shared node must fail");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.message().contains("2 owners")));

        let mut reversed_siblings = valid_raw(SOURCE);
        reversed_siblings.files[0].functions[0].body.expressions.swap(0, 1);
        let RawExpressionKind::Addition { lhs, rhs, .. } =
            &mut reversed_siblings.files[0].functions[0].body.expressions[2].kind
        else {
            unreachable!("fixture root must be an addition");
        };
        *lhs = 1;
        *rhs = 0;
        let diagnostics = verify_snapshot(reversed_siblings, &sources)
            .expect_err("noncanonical sibling order must fail");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.message().contains("expression arena is not canonical postorder")
        }));
    }

    fn deep_fixture(depth: u32) -> (String, RawProjectSyntaxSnapshot) {
        assert!(depth >= 1);
        let prefix = "export function deep(): i32 { return ";
        let mut expression_text = "1".to_owned();
        let mut expressions = vec![RawExpressionSyntax {
            span: raw_span(0, prefix.len(), prefix.len() + 1),
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        }];
        let mut root = 0_u32;
        for _ in 2..=depth {
            let operator_start = prefix.len() + expression_text.len() + 1;
            expression_text.push_str(" + 1");
            let literal_start = prefix.len() + expression_text.len() - 1;
            let literal_id = u32::try_from(expressions.len()).expect("fixture id must fit");
            expressions.push(RawExpressionSyntax {
                span: raw_span(0, literal_start, literal_start + 1),
                kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
            });
            let add_id = u32::try_from(expressions.len()).expect("fixture id must fit");
            expressions.push(RawExpressionSyntax {
                span: raw_span(0, prefix.len(), prefix.len() + expression_text.len()),
                kind: RawExpressionKind::Addition {
                    operator_span: raw_span(0, operator_start, operator_start + 1),
                    lhs: root,
                    rhs: literal_id,
                },
            });
            root = add_id;
        }
        let source = format!("{prefix}{expression_text}; }}");
        let (name_start, name_end) = range_from(&source, "deep", 0);
        let (result_start, result_end) = range_from(&source, "i32", name_end);
        let (body_start, _) = range_from(&source, "{", result_end);
        let (return_start, return_end) = range_from(&source, "return", body_start);
        let (semicolon_start, semicolon_end) = range_from(&source, ";", return_end);
        let _ = semicolon_start;
        let raw = RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                functions: vec![RawFunctionSyntax {
                    span: raw_span(0, 0, source.len()),
                    export_span: raw_span(0, 0, 6),
                    function_span: raw_span(0, 7, 15),
                    name: RawIdentifierSyntax {
                        text: "deep".to_owned(),
                        span: raw_span(0, name_start, name_end),
                    },
                    parameters: Vec::new(),
                    result_type: RawTypeSyntax {
                        span: raw_span(0, result_start, result_end),
                        kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                    },
                    body: RawFunctionBodySyntax {
                        span: raw_span(0, body_start, source.len()),
                        statements: vec![RawStatementSyntax {
                            span: raw_span(0, return_start, semicolon_end),
                            kind: RawStatementKind::Return {
                                keyword_span: raw_span(0, return_start, return_end),
                                value: root,
                            },
                        }],
                        expressions,
                    },
                }],
            }],
            diagnostics: Vec::new(),
        };
        (source, raw)
    }

    #[test]
    fn expression_depth_accepts_the_limit_and_rejects_one_more() {
        let (source, raw) = deep_fixture(MAX_EXPRESSION_DEPTH);
        assert!(verify_snapshot(raw, &sources(&source)).is_ok());

        let (source, raw) = deep_fixture(MAX_EXPRESSION_DEPTH + 1);
        let diagnostics =
            verify_snapshot(raw, &sources(&source)).expect_err("depth overflow must fail");
        assert!(has_code(&diagnostics, "ZRYNA-Y1003"));
    }

    #[test]
    fn deterministic_error_selection_reserves_a_terminal_budget_diagnostic() {
        let sources = sources(SOURCE);
        let mut raw = valid_raw(SOURCE);
        let mut invalid = raw.files[0].functions[0].clone();
        invalid.span.file = 1;
        raw.files[0].functions = vec![invalid; 300];
        let diagnostics = verify_snapshot(raw, &sources).expect_err("invalid functions must fail");
        assert_eq!(diagnostics.len(), MAX_VALIDATION_ERRORS);
        assert_eq!(diagnostics.last().map(Diagnostic::code), Some("ZRYNA-Y1003"));
        assert!(
            diagnostics
                .last()
                .is_some_and(|diagnostic| diagnostic.message().contains("diagnostic limit"))
        );
    }

    #[test]
    fn parameter_ranges_follow_the_function_name() {
        let sources = sources(SOURCE);
        let mut raw = valid_raw(SOURCE);
        raw.files[0].functions[0].parameters[0].span.start = 0;

        let diagnostics =
            verify_snapshot(raw, &sources).expect_err("parameter before function name must fail");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.message().contains("function name is not before first parameter")
        }));
    }

    #[test]
    fn shuffled_invalid_files_produce_identical_diagnostics() {
        let sources = SourceMap::build(vec![
            SourceFileInput { path: "src/a.zry".to_owned(), text: String::new() },
            SourceFileInput { path: "src/z.zry".to_owned(), text: String::new() },
        ])
        .expect("fixture source map must be valid");
        let files = vec![
            RawSourceUnit { id: 0, path: "src/z.zry".to_owned(), functions: Vec::new() },
            RawSourceUnit { id: 0, path: "src/a.zry".to_owned(), functions: Vec::new() },
        ];
        let first = verify_snapshot(
            RawProjectSyntaxSnapshot {
                schema_version: PROTOCOL_VERSION,
                files: files.clone(),
                diagnostics: Vec::new(),
            },
            &sources,
        )
        .expect_err("mismatched duplicate files must fail");
        let second = verify_snapshot(
            RawProjectSyntaxSnapshot {
                schema_version: PROTOCOL_VERSION,
                files: files.into_iter().rev().collect(),
                diagnostics: Vec::new(),
            },
            &sources,
        )
        .expect_err("mismatched duplicate files must fail");
        assert_eq!(first, second);
    }

    #[test]
    fn equal_identity_duplicates_and_provider_diagnostics_are_order_independent() {
        let sources = sources(SOURCE);
        let valid = valid_raw(SOURCE).files.remove(0);
        let mut malformed = valid.clone();
        malformed.functions[0].span.file = 1;
        let files = vec![valid, malformed];

        let verify_files = |files| {
            verify_snapshot(
                RawProjectSyntaxSnapshot {
                    schema_version: PROTOCOL_VERSION,
                    files,
                    diagnostics: Vec::new(),
                },
                &sources,
            )
            .expect_err("duplicate file identity must fail")
        };
        assert_eq!(verify_files(files.clone()), verify_files(files.into_iter().rev().collect()));

        let mut first = valid_raw(SOURCE);
        let mut alternate = first.diagnostics[0].clone();
        alternate.severity = Severity::Error;
        alternate.location = RawDiagnosticLocation::Global;
        first.diagnostics.push(alternate);
        let mut second = first.clone();
        second.diagnostics.reverse();
        let first = verify_snapshot(first, &sources).expect("provider diagnostics must verify");
        let second = verify_snapshot(second, &sources).expect("provider diagnostics must verify");
        assert_eq!(first.diagnostics(), second.diagnostics());
    }

    #[test]
    fn checked_in_schema_and_wire_fixtures_match_the_runtime_contract() {
        const VALID: &str = include_str!("../../../tests/fixtures/syntax-v2-valid.json");
        const UNKNOWN: &str = include_str!("../../../tests/fixtures/syntax-v2-unknown-field.json");
        const MISSING: &str = include_str!("../../../tests/fixtures/syntax-v2-missing-field.json");
        const SCHEMA: &str = include_str!("../../../schemas/zryna-syntax-v2.schema.json");

        let raw = decode_snapshot(VALID.as_bytes()).expect("golden fixture must decode");
        let source = "export function yes(): bool { return true; }";
        let project =
            verify_snapshot(raw.clone(), &sources(source)).expect("golden fixture must verify");
        assert_eq!(project.schema_version(), PROTOCOL_VERSION);
        assert_eq!(
            serde_json::to_value(raw).expect("raw fixture must serialize"),
            serde_json::from_str::<serde_json::Value>(VALID).expect("fixture JSON must parse")
        );
        assert!(decode_snapshot(UNKNOWN.as_bytes()).is_err());
        assert!(decode_snapshot(MISSING.as_bytes()).is_err());

        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("checked-in schema must be valid JSON");
        assert_eq!(schema["properties"]["schema_version"]["const"], PROTOCOL_VERSION);
        assert_eq!(schema["properties"]["files"]["maxItems"], MAX_SOURCE_FILES);
        assert_eq!(schema["properties"]["diagnostics"]["maxItems"], MAX_PROVIDER_DIAGNOSTICS);
        assert_eq!(
            schema["$defs"]["sourceUnit"]["properties"]["path"]["maxLength"],
            zryna_source::MAX_SOURCE_PATH_BYTES
        );
        assert_eq!(
            schema["$defs"]["sourceUnit"]["properties"]["functions"]["maxItems"],
            MAX_FUNCTIONS_PER_FILE
        );
        assert_eq!(
            schema["$defs"]["identifier"]["properties"]["text"]["maxLength"],
            MAX_NAME_CHARACTERS
        );
        assert_eq!(
            schema["$defs"]["function"]["properties"]["parameters"]["maxItems"],
            MAX_PARAMETERS_PER_FUNCTION
        );
        assert_eq!(
            schema["$defs"]["body"]["properties"]["statements"]["maxItems"],
            MAX_STATEMENTS_PER_FUNCTION
        );
        assert_eq!(
            schema["$defs"]["body"]["properties"]["expressions"]["maxItems"],
            MAX_EXPRESSIONS_PER_FUNCTION
        );
        assert_eq!(
            schema["$defs"]["expressionKind"]["oneOf"][2]["properties"]["spelling"]["maxLength"],
            MAX_LITERAL_BYTES
        );
        assert_eq!(
            schema["$defs"]["diagnostic"]["properties"]["message"]["maxLength"],
            MAX_DIAGNOSTIC_TEXT_CHARACTERS
        );
    }
}
