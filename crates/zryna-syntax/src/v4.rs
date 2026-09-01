//! Fail-closed provider-neutral syntax protocol version 4.

#![allow(missing_docs)]
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::drop_non_drop,
    clippy::manual_let_else,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::too_many_lines
)]

use std::{collections::BTreeSet, fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_source::{
    FileId, MAX_SOURCE_FILES, NormalizedSourcePath, SourceMap, SourceMapIdentity, Span,
    UntrustedSpan,
};

pub const PROTOCOL_VERSION: u32 = 4;
pub const MAX_RESPONSE_BYTES: usize = 64 * 1_024 * 1_024;
pub const MAX_AGGREGATE_SOURCE_BYTES: usize = 8 * 1_024 * 1_024;
pub const MAX_IMPORTS_PER_MODULE: usize = 4_096;
pub const MAX_IMPORTS_PER_PROJECT: usize = 65_536;
pub const MAX_IMPORTED_NAMES_PER_DECLARATION: usize = 256;
pub const MAX_IMPORTED_NAMES_PER_PROJECT: usize = 65_536;
pub const MAX_DATA_DECLARATIONS_PER_MODULE: usize = 4_096;
pub const MAX_DATA_DECLARATIONS_PER_PROJECT: usize = 16_384;
pub const MAX_MEMBERS_PER_DECLARATION: usize = 1_024;
pub const MAX_MEMBERS_PER_PROJECT: usize = 65_536;
pub const MAX_TYPE_NODES_PER_MODULE: usize = 65_536;
pub const MAX_TYPE_NODES_PER_PROJECT: usize = 262_144;
pub const MAX_FUNCTIONS_PER_MODULE: usize = 4_096;
pub const MAX_FUNCTIONS_PER_PROJECT: usize = 16_384;
pub const MAX_PARAMETERS_PER_FUNCTION: usize = 256;
pub const MAX_PARAMETERS_PER_PROJECT: usize = 262_144;
pub const MAX_BLOCKS_PER_FUNCTION: usize = 4_096;
pub const MAX_BLOCKS_PER_PROJECT: usize = 65_536;
pub const MAX_STATEMENTS_PER_FUNCTION: usize = 4_096;
pub const MAX_STATEMENTS_PER_PROJECT: usize = 65_536;
pub const MAX_EXPRESSIONS_PER_FUNCTION: usize = 16_384;
pub const MAX_EXPRESSIONS_PER_PROJECT: usize = 262_144;
pub const MAX_INITIALIZERS_PER_CONSTRUCTION: usize = 1_024;
pub const MAX_ELEMENTS_PER_CONSTRUCTION: usize = 4_096;
pub const MAX_AGGREGATE_OPERANDS_PER_PROJECT: usize = 65_536;
pub const MAX_MATCH_ARMS_PER_EXPRESSION: usize = 1_024;
pub const MAX_MATCH_ARMS_PER_PROJECT: usize = 65_536;
pub const MAX_FIXED_ARRAY_LENGTH: u32 = 1_048_576;
pub const MAX_NESTING_DEPTH: u32 = 128;
pub const MAX_PROVIDER_DIAGNOSTICS: usize = 256;
pub const MAX_VALIDATION_ERRORS: usize = 256;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProjectSyntaxSnapshot {
    pub schema_version: u32,
    #[serde(deserialize_with = "files")]
    pub files: Vec<RawSourceUnit>,
    #[serde(deserialize_with = "diagnostics")]
    pub diagnostics: Vec<RawProviderDiagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawSourceUnit {
    pub id: u32,
    pub path: String,
    #[serde(deserialize_with = "imports")]
    pub imports: Vec<RawImportSyntax>,
    #[serde(deserialize_with = "types")]
    pub type_syntax: Vec<RawTypeSyntax>,
    #[serde(deserialize_with = "declarations")]
    pub data_declarations: Vec<RawDataDeclaration>,
    #[serde(deserialize_with = "functions")]
    pub functions: Vec<RawFunctionSyntax>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawIdentifierSyntax {
    pub text: String,
    pub span: UntrustedSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawImportSyntax {
    pub span: UntrustedSpan,
    pub import_span: UntrustedSpan,
    #[serde(deserialize_with = "bindings")]
    pub bindings: Vec<RawImportBindingSyntax>,
    pub from_span: UntrustedSpan,
    pub specifier: RawModuleSpecifierSyntax,
    pub semicolon_span: UntrustedSpan,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawImportBindingSyntax {
    pub span: UntrustedSpan,
    pub imported: RawIdentifierSyntax,
    pub local: RawIdentifierSyntax,
    pub as_span: Option<UntrustedSpan>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawModuleSpecifierSyntax {
    pub text: String,
    pub token_span: UntrustedSpan,
    pub value_span: UntrustedSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawTypeSyntax {
    pub span: UntrustedSpan,
    pub kind: RawTypeSyntaxKind,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawTypeSyntaxKind {
    Missing,
    Named {
        name: RawIdentifierSyntax,
    },
    String {
        keyword_span: UntrustedSpan,
    },
    Vec {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        argument: u32,
        greater_than_span: UntrustedSpan,
    },
    Shared {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        argument: u32,
        greater_than_span: UntrustedSpan,
    },
    Weak {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        argument: u32,
        greater_than_span: UntrustedSpan,
    },
    Borrow {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        argument: u32,
        greater_than_span: UntrustedSpan,
    },
    BorrowMut {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        argument: u32,
        greater_than_span: UntrustedSpan,
    },
    FixedArray {
        keyword_span: UntrustedSpan,
        less_than_span: UntrustedSpan,
        element: u32,
        comma_span: UntrustedSpan,
        length_span: UntrustedSpan,
        length_spelling: String,
        length: u32,
        greater_than_span: UntrustedSpan,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawDataDeclaration {
    pub span: UntrustedSpan,
    pub export_span: Option<UntrustedSpan>,
    pub kind: RawDataDeclarationKind,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawDataDeclarationKind {
    Struct {
        interface_span: UntrustedSpan,
        name: RawIdentifierSyntax,
        extends_span: UntrustedSpan,
        marker_span: UntrustedSpan,
        open_brace_span: UntrustedSpan,
        #[serde(deserialize_with = "fields")]
        fields: Vec<RawDataField>,
        close_brace_span: UntrustedSpan,
    },
    Enum {
        interface_span: UntrustedSpan,
        name: RawIdentifierSyntax,
        extends_span: UntrustedSpan,
        marker_span: UntrustedSpan,
        open_brace_span: UntrustedSpan,
        #[serde(deserialize_with = "variants")]
        variants: Vec<RawEnumVariant>,
        close_brace_span: UntrustedSpan,
    },
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawDataField {
    pub span: UntrustedSpan,
    pub name: RawIdentifierSyntax,
    pub colon_span: UntrustedSpan,
    pub type_syntax: u32,
    pub semicolon_span: UntrustedSpan,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawEnumVariant {
    pub span: UntrustedSpan,
    pub name: RawIdentifierSyntax,
    pub colon_span: UntrustedSpan,
    pub payload_type: Option<u32>,
    pub none_span: Option<UntrustedSpan>,
    pub semicolon_span: UntrustedSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionSyntax {
    pub span: UntrustedSpan,
    pub export_span: Option<UntrustedSpan>,
    pub function_span: UntrustedSpan,
    pub name: RawIdentifierSyntax,
    #[serde(deserialize_with = "parameters")]
    pub parameters: Vec<RawParameterSyntax>,
    pub result_type: u32,
    pub body: RawFunctionBodySyntax,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawParameterSyntax {
    pub span: UntrustedSpan,
    pub name: RawIdentifierSyntax,
    pub type_syntax: u32,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionBodySyntax {
    pub span: UntrustedSpan,
    pub root_block: u32,
    #[serde(deserialize_with = "blocks")]
    pub blocks: Vec<RawBlockSyntax>,
    #[serde(deserialize_with = "statements")]
    pub statements: Vec<RawStatementSyntax>,
    #[serde(deserialize_with = "expressions")]
    pub expressions: Vec<RawExpressionSyntax>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawBlockSyntax {
    pub span: UntrustedSpan,
    pub open_brace_span: UntrustedSpan,
    #[serde(deserialize_with = "statement_ids")]
    pub statements: Vec<u32>,
    pub close_brace_span: UntrustedSpan,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawStatementSyntax {
    pub span: UntrustedSpan,
    pub kind: RawStatementKind,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawStatementKind {
    LocalDeclaration {
        keyword_span: UntrustedSpan,
        mutable: bool,
        name: RawIdentifierSyntax,
        type_syntax: u32,
        equals_span: UntrustedSpan,
        initializer: u32,
        semicolon_span: UntrustedSpan,
    },
    Assignment {
        target: u32,
        equals_span: UntrustedSpan,
        value: u32,
        semicolon_span: UntrustedSpan,
    },
    Return {
        keyword_span: UntrustedSpan,
        value: u32,
        semicolon_span: UntrustedSpan,
    },
    Block {
        block: u32,
    },
    If {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        condition: u32,
        close_paren_span: UntrustedSpan,
        then_block: u32,
        else_clause: Option<RawElseSyntax>,
    },
    While {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        condition: u32,
        close_paren_span: UntrustedSpan,
        body_block: u32,
    },
    ExpressionStatement {
        expression: u32,
        semicolon_span: UntrustedSpan,
    },
    WeakUpgrade {
        keyword_span: UntrustedSpan,
        weak: u32,
        as_span: UntrustedSpan,
        binding: RawIdentifierSyntax,
        success_block: u32,
        else_span: UntrustedSpan,
        failure_block: u32,
    },
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawElseSyntax {
    pub keyword_span: UntrustedSpan,
    pub block: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawExpressionSyntax {
    pub span: UntrustedSpan,
    pub kind: RawExpressionKind,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawExpressionKind {
    Reference {
        name: RawIdentifierSyntax,
    },
    BoolLiteral {
        value: bool,
    },
    I32Literal {
        spelling: String,
    },
    StringLiteral {
        spelling: String,
    },
    Negation {
        operator_span: UntrustedSpan,
        operand: u32,
    },
    Addition {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    Subtraction {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    Multiplication {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    Equal {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    NotEqual {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    LessThan {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    LessEqual {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    GreaterThan {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    GreaterEqual {
        operator_span: UntrustedSpan,
        lhs: u32,
        rhs: u32,
    },
    Call {
        callee: RawIdentifierSyntax,
        open_paren_span: UntrustedSpan,
        #[serde(deserialize_with = "arguments")]
        arguments: Vec<u32>,
        close_paren_span: UntrustedSpan,
    },
    StructConstruction {
        type_name: RawIdentifierSyntax,
        open_paren_span: UntrustedSpan,
        open_brace_span: UntrustedSpan,
        #[serde(deserialize_with = "initializers")]
        fields: Vec<RawFieldInitializer>,
        close_brace_span: UntrustedSpan,
        close_paren_span: UntrustedSpan,
    },
    EnumConstruction {
        type_name: RawIdentifierSyntax,
        dot_span: UntrustedSpan,
        variant: RawIdentifierSyntax,
        open_paren_span: UntrustedSpan,
        payload: Option<u32>,
        close_paren_span: UntrustedSpan,
    },
    FixedArrayConstruction {
        type_syntax: u32,
        open_paren_span: UntrustedSpan,
        open_bracket_span: UntrustedSpan,
        #[serde(deserialize_with = "elements")]
        elements: Vec<u32>,
        close_bracket_span: UntrustedSpan,
        close_paren_span: UntrustedSpan,
    },
    VecConstruction {
        type_syntax: u32,
        open_paren_span: UntrustedSpan,
        open_bracket_span: UntrustedSpan,
        #[serde(deserialize_with = "elements")]
        elements: Vec<u32>,
        close_bracket_span: UntrustedSpan,
        close_paren_span: UntrustedSpan,
    },
    FieldAccess {
        base: u32,
        dot_span: UntrustedSpan,
        field: RawIdentifierSyntax,
    },
    Index {
        base: u32,
        open_bracket_span: UntrustedSpan,
        index: u32,
        close_bracket_span: UntrustedSpan,
    },
    Clone {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    Shared {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    Downgrade {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    Borrow {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    BorrowMut {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    VecPush {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        vector: u32,
        comma_span: UntrustedSpan,
        value: u32,
        close_paren_span: UntrustedSpan,
    },
    Match {
        keyword_span: UntrustedSpan,
        open_paren_span: UntrustedSpan,
        scrutinee: u32,
        close_paren_span: UntrustedSpan,
        open_brace_span: UntrustedSpan,
        #[serde(deserialize_with = "arms")]
        arms: Vec<RawMatchArm>,
        close_brace_span: UntrustedSpan,
    },
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFieldInitializer {
    pub span: UntrustedSpan,
    pub kind: RawFieldInitializerKind,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawFieldInitializerKind {
    Shorthand { name: RawIdentifierSyntax, value: u32 },
    Explicit { name: RawIdentifierSyntax, colon_span: UntrustedSpan, value: u32 },
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawMatchArm {
    pub span: UntrustedSpan,
    pub type_name: RawIdentifierSyntax,
    pub dot_span: UntrustedSpan,
    pub variant: RawIdentifierSyntax,
    pub binding: Option<RawIdentifierSyntax>,
    pub arrow_span: UntrustedSpan,
    pub value: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProviderDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub location: RawDiagnosticLocation,
    pub message: String,
    pub guidance: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawDiagnosticLocation {
    Global,
    Source { span: UntrustedSpan },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxDecodeError {
    ResponseTooLarge { actual: usize, limit: usize },
    InvalidSnapshot,
}
impl SyntaxDecodeError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ResponseTooLarge { .. } => "ZRYNA-F1401",
            Self::InvalidSnapshot => "ZRYNA-Y4001",
        }
    }
}
impl fmt::Display for SyntaxDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseTooLarge { actual, limit } => {
                write!(f, "syntax response contains {actual} bytes; the limit is {limit}")
            }
            Self::InvalidSnapshot => {
                f.write_str("syntax response is not exact bounded protocol-v4 JSON")
            }
        }
    }
}
impl std::error::Error for SyntaxDecodeError {}

fn bounded<'de, D, T, const MAX: usize>(d: D, label: &'static str) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct B<T, const MAX: usize>(&'static str, PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for B<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {MAX} {}", self.0)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(MAX));
            while out.len() < MAX {
                match seq.next_element()? {
                    Some(v) => out.push(v),
                    None => return Ok(out),
                }
            }
            if seq.next_element::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom(format_args!("{} exceeds limit {MAX}", self.0)));
            }
            Ok(out)
        }
    }
    d.deserialize_seq(B::<T, MAX>(label, PhantomData))
}
macro_rules! bounded_fn {
    ($name:ident, $ty:ty, $max:expr, $label:literal) => {
        fn $name<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<$ty>, D::Error> {
            bounded::<D, $ty, $max>(d, $label)
        }
    };
}
bounded_fn!(files, RawSourceUnit, MAX_SOURCE_FILES, "files");
bounded_fn!(diagnostics, RawProviderDiagnostic, MAX_PROVIDER_DIAGNOSTICS, "diagnostics");
bounded_fn!(imports, RawImportSyntax, MAX_IMPORTS_PER_MODULE, "imports");
bounded_fn!(bindings, RawImportBindingSyntax, MAX_IMPORTED_NAMES_PER_DECLARATION, "bindings");
bounded_fn!(types, RawTypeSyntax, MAX_TYPE_NODES_PER_MODULE, "type nodes");
bounded_fn!(
    declarations,
    RawDataDeclaration,
    MAX_DATA_DECLARATIONS_PER_MODULE,
    "data declarations"
);
bounded_fn!(fields, RawDataField, MAX_MEMBERS_PER_DECLARATION, "fields");
bounded_fn!(variants, RawEnumVariant, MAX_MEMBERS_PER_DECLARATION, "variants");
bounded_fn!(functions, RawFunctionSyntax, MAX_FUNCTIONS_PER_MODULE, "functions");
bounded_fn!(parameters, RawParameterSyntax, MAX_PARAMETERS_PER_FUNCTION, "parameters");
bounded_fn!(blocks, RawBlockSyntax, MAX_BLOCKS_PER_FUNCTION, "blocks");
bounded_fn!(statements, RawStatementSyntax, MAX_STATEMENTS_PER_FUNCTION, "statements");
bounded_fn!(statement_ids, u32, MAX_STATEMENTS_PER_FUNCTION, "statement ids");
bounded_fn!(expressions, RawExpressionSyntax, MAX_EXPRESSIONS_PER_FUNCTION, "expressions");
bounded_fn!(arguments, u32, MAX_PARAMETERS_PER_FUNCTION, "arguments");
bounded_fn!(
    initializers,
    RawFieldInitializer,
    MAX_INITIALIZERS_PER_CONSTRUCTION,
    "field initializers"
);
bounded_fn!(elements, u32, MAX_ELEMENTS_PER_CONSTRUCTION, "elements");
bounded_fn!(arms, RawMatchArm, MAX_MATCH_ARMS_PER_EXPRESSION, "match arms");

/// Decodes one exact, resource-bounded protocol-v4 JSON response.
///
/// # Errors
///
/// Returns a stable decode failure when the byte ceiling is exceeded or the response is not the
/// closed protocol-v4 DTO grammar.
pub fn decode_snapshot(bytes: &[u8]) -> Result<RawProjectSyntaxSnapshot, SyntaxDecodeError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(SyntaxDecodeError::ResponseTooLarge {
            actual: bytes.len(),
            limit: MAX_RESPONSE_BYTES,
        });
    }
    reject_duplicate_json_keys(bytes)?;
    serde_json::from_slice(bytes).map_err(|_| SyntaxDecodeError::InvalidSnapshot)
}

struct DuplicateCheckedValue;
impl<'de> Deserialize<'de> for DuplicateCheckedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DuplicateCheckedVisitor;
        impl<'de> Visitor<'de> for DuplicateCheckedVisitor {
            type Value = DuplicateCheckedValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }
            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_string<E>(self, _: String) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue)
            }
            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                DuplicateCheckedValue::deserialize(deserializer)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                while sequence.next_element::<DuplicateCheckedValue>()?.is_some() {}
                Ok(DuplicateCheckedValue)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !keys.insert(key) {
                        return Err(A::Error::custom("duplicate JSON object key"));
                    }
                    map.next_value::<DuplicateCheckedValue>()?;
                }
                Ok(DuplicateCheckedValue)
            }
        }
        deserializer.deserialize_any(DuplicateCheckedVisitor)
    }
}
fn reject_duplicate_json_keys(bytes: &[u8]) -> Result<(), SyntaxDecodeError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    DuplicateCheckedValue::deserialize(&mut deserializer)
        .map_err(|_| SyntaxDecodeError::InvalidSnapshot)?;
    deserializer.end().map_err(|_| SyntaxDecodeError::InvalidSnapshot)
}

/// One source unit whose complete v4 claim has been authenticated.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceUnit {
    id: FileId,
    path: NormalizedSourcePath,
    raw: RawSourceUnit,
}
impl SourceUnit {
    #[must_use]
    pub const fn id(&self) -> FileId {
        self.id
    }
    #[must_use]
    pub const fn path(&self) -> &NormalizedSourcePath {
        &self.path
    }
    #[must_use]
    pub fn imports(&self) -> &[RawImportSyntax] {
        &self.raw.imports
    }
    #[must_use]
    pub fn type_syntax(&self) -> &[RawTypeSyntax] {
        &self.raw.type_syntax
    }
    #[must_use]
    pub fn data_declarations(&self) -> &[RawDataDeclaration] {
        &self.raw.data_declarations
    }
    #[must_use]
    pub fn functions(&self) -> &[RawFunctionSyntax] {
        &self.raw.functions
    }
}

/// Opaque all-or-nothing v4 snapshot bound to one exact source map identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectSyntaxSnapshot {
    #[serde(skip)]
    source_map_identity: SourceMapIdentity,
    schema_version: u32,
    files: Vec<SourceUnit>,
    diagnostics: Vec<Diagnostic>,
}
impl ProjectSyntaxSnapshot {
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    #[must_use]
    pub fn files(&self) -> &[SourceUnit] {
        &self.files
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[must_use]
    pub fn is_bound_to(&self, sources: &SourceMap) -> bool {
        self.source_map_identity == sources.identity()
            && self.files.len() == sources.len()
            && self.files.iter().all(|file| {
                sources.source(file.id).is_some_and(|source| source.path() == &file.path)
            })
    }
}

#[derive(Default)]
struct Counts {
    imports: usize,
    bindings: usize,
    declarations: usize,
    members: usize,
    types: usize,
    functions: usize,
    parameters: usize,
    blocks: usize,
    statements: usize,
    expressions: usize,
    aggregate_operands: usize,
    match_arms: usize,
}
impl Counts {
    fn add(slot: &mut usize, value: usize) -> bool {
        if let Some(next) = slot.checked_add(value) {
            *slot = next;
            true
        } else {
            false
        }
    }
    fn exceeded(&self) -> bool {
        self.imports > MAX_IMPORTS_PER_PROJECT
            || self.bindings > MAX_IMPORTED_NAMES_PER_PROJECT
            || self.declarations > MAX_DATA_DECLARATIONS_PER_PROJECT
            || self.members > MAX_MEMBERS_PER_PROJECT
            || self.types > MAX_TYPE_NODES_PER_PROJECT
            || self.functions > MAX_FUNCTIONS_PER_PROJECT
            || self.parameters > MAX_PARAMETERS_PER_PROJECT
            || self.blocks > MAX_BLOCKS_PER_PROJECT
            || self.statements > MAX_STATEMENTS_PER_PROJECT
            || self.expressions > MAX_EXPRESSIONS_PER_PROJECT
            || self.aggregate_operands > MAX_AGGREGATE_OPERANDS_PER_PROJECT
            || self.match_arms > MAX_MATCH_ARMS_PER_PROJECT
    }
}

#[derive(Default)]
struct Errors {
    items: Vec<Diagnostic>,
    truncated: bool,
}
impl Errors {
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.items.len() < MAX_VALIDATION_ERRORS - 1 {
            self.items.push(diagnostic);
        } else {
            self.truncated = true;
        }
    }
    fn protocol(&mut self, path: Option<&str>, message: impl Into<String>) {
        self.push(Diagnostic::error(
            "ZRYNA-Y4001",
            path.map(str::to_owned),
            message,
            "return the exact bounded protocol-v4 contract",
        ));
    }
    fn node(&mut self, path: &NormalizedSourcePath, message: impl Into<String>) {
        self.push(Diagnostic::error(
            "ZRYNA-Y4002",
            Some(path.as_str().to_owned()),
            message,
            "return source-faithful canonical protocol-v4 syntax",
        ));
    }
    fn limit(&mut self, message: impl Into<String>) {
        self.push(limit_error(message));
    }
    fn finish(mut self) -> Vec<Diagnostic> {
        self.items.sort_by_key(ToString::to_string);
        if self.truncated {
            self.items.push(limit_error("validation diagnostics exceeded the deterministic limit"));
        }
        self.items
    }
}
fn limit_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error("ZRYNA-F1401", None, message, "reduce the bounded protocol-v4 input")
}

/// Authenticates a bounded raw v4 response against one exact final source map.
///
/// # Errors
///
/// Returns deterministic bounded protocol, source-fidelity, or resource diagnostics. No verified
/// view is returned unless every response claim succeeds.
pub fn verify_snapshot(
    raw: RawProjectSyntaxSnapshot,
    sources: &SourceMap,
) -> Result<ProjectSyntaxSnapshot, Vec<Diagnostic>> {
    let mut errors = Errors::default();
    if raw.schema_version != PROTOCOL_VERSION {
        errors.protocol(None, "snapshot schema version is not exactly 4");
    }
    if raw.files.len() != sources.len() {
        errors.protocol(None, "snapshot file set is not complete");
    }
    if let Err(message) = check_budgets(&raw, sources) {
        return Err(vec![limit_error(message)]);
    }
    let mut verified = Vec::with_capacity(raw.files.len());
    for (position, file) in raw.files.into_iter().enumerate() {
        if let Some(file) = verify_file(file, position, sources, &mut errors) {
            verified.push(file);
        }
    }
    let diagnostics = verify_provider_diagnostics(raw.diagnostics, sources, &mut errors);
    if errors.items.is_empty() {
        Ok(ProjectSyntaxSnapshot {
            source_map_identity: sources.identity(),
            schema_version: PROTOCOL_VERSION,
            files: verified,
            diagnostics,
        })
    } else {
        Err(errors.finish())
    }
}

fn check_budgets(raw: &RawProjectSyntaxSnapshot, sources: &SourceMap) -> Result<(), &'static str> {
    if raw.files.len() > MAX_SOURCE_FILES {
        return Err("syntax snapshot exceeds the source-file limit");
    }
    if raw.diagnostics.len() > MAX_PROVIDER_DIAGNOSTICS {
        return Err("provider diagnostics exceed the protocol-v4 limit");
    }
    let mut source_bytes = 0usize;
    for index in 0..sources.len() {
        let id = sources
            .verify_file_id(u32::try_from(index).map_err(|_| "source id overflow")?)
            .map_err(|_| "source id is not canonical")?;
        let source = sources.source(id).ok_or("source file is unavailable")?;
        source_bytes =
            source_bytes.checked_add(source.text().len()).ok_or("source byte count overflow")?;
    }
    if source_bytes > MAX_AGGREGATE_SOURCE_BYTES {
        return Err("source map exceeds the protocol-v4 aggregate byte limit");
    }
    let mut c = Counts::default();
    for file in &raw.files {
        if file.imports.len() > MAX_IMPORTS_PER_MODULE
            || file.type_syntax.len() > MAX_TYPE_NODES_PER_MODULE
            || file.data_declarations.len() > MAX_DATA_DECLARATIONS_PER_MODULE
            || file.functions.len() > MAX_FUNCTIONS_PER_MODULE
        {
            return Err("one module exceeds a protocol-v4 collection limit");
        }
        if !Counts::add(&mut c.imports, file.imports.len())
            || !Counts::add(&mut c.declarations, file.data_declarations.len())
            || !Counts::add(&mut c.types, file.type_syntax.len())
            || !Counts::add(&mut c.functions, file.functions.len())
        {
            return Err("project count overflow");
        }
        for import in &file.imports {
            if import.bindings.is_empty()
                || import.bindings.len() > MAX_IMPORTED_NAMES_PER_DECLARATION
                || !Counts::add(&mut c.bindings, import.bindings.len())
            {
                return Err("import binding count overflow");
            }
        }
        for declaration in &file.data_declarations {
            let n = match &declaration.kind {
                RawDataDeclarationKind::Struct { fields, .. } => fields.len(),
                RawDataDeclarationKind::Enum { variants, .. } => variants.len(),
            };
            if n == 0 || n > MAX_MEMBERS_PER_DECLARATION || !Counts::add(&mut c.members, n) {
                return Err("data member count overflow");
            }
        }
        for function in &file.functions {
            if function.parameters.len() > MAX_PARAMETERS_PER_FUNCTION
                || function.body.blocks.is_empty()
                || function.body.blocks.len() > MAX_BLOCKS_PER_FUNCTION
                || function.body.statements.len() > MAX_STATEMENTS_PER_FUNCTION
                || function.body.expressions.len() > MAX_EXPRESSIONS_PER_FUNCTION
                || function
                    .body
                    .blocks
                    .iter()
                    .any(|block| block.statements.len() > MAX_STATEMENTS_PER_FUNCTION)
            {
                return Err("one function exceeds a protocol-v4 collection limit");
            }
            if !Counts::add(&mut c.parameters, function.parameters.len())
                || !Counts::add(&mut c.blocks, function.body.blocks.len())
                || !Counts::add(&mut c.statements, function.body.statements.len())
                || !Counts::add(&mut c.expressions, function.body.expressions.len())
            {
                return Err("function arena count overflow");
            }
            for expression in &function.body.expressions {
                match &expression.kind {
                    RawExpressionKind::Call { arguments, .. }
                        if arguments.len() > MAX_PARAMETERS_PER_FUNCTION =>
                    {
                        return Err("call argument count exceeds its protocol-v4 limit");
                    }
                    RawExpressionKind::StructConstruction { fields, .. } => {
                        if fields.len() > MAX_INITIALIZERS_PER_CONSTRUCTION {
                            return Err("construction field count exceeds its protocol-v4 limit");
                        }
                        if !Counts::add(&mut c.aggregate_operands, fields.len()) {
                            return Err("initializer count overflow");
                        }
                    }
                    RawExpressionKind::FixedArrayConstruction { elements, .. }
                    | RawExpressionKind::VecConstruction { elements, .. } => {
                        if elements.len() > MAX_ELEMENTS_PER_CONSTRUCTION {
                            return Err("construction element count exceeds its protocol-v4 limit");
                        }
                        if !Counts::add(&mut c.aggregate_operands, elements.len()) {
                            return Err("element count overflow");
                        }
                    }
                    RawExpressionKind::Match { arms, .. } => {
                        if arms.len() > MAX_MATCH_ARMS_PER_EXPRESSION {
                            return Err("match arm count exceeds its protocol-v4 limit");
                        }
                        if !Counts::add(&mut c.match_arms, arms.len()) {
                            return Err("match arm count overflow");
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if c.exceeded() {
        Err("syntax snapshot exceeds an aggregate protocol-v4 limit")
    } else {
        Ok(())
    }
}

fn verify_file(
    raw: RawSourceUnit,
    position: usize,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<SourceUnit> {
    let expected = u32::try_from(position).ok()?;
    if raw.id != expected {
        errors.protocol(None, "source units are not in canonical dense file-id order");
        return None;
    }
    let id = sources.verify_file_id(raw.id).ok()?;
    let path = match NormalizedSourcePath::new(raw.path.clone()) {
        Ok(path) => path,
        Err(_) => {
            errors.protocol(None, "source unit path is not portable and normalized");
            return None;
        }
    };
    if sources.source(id).is_none_or(|source| source.path() != &path) {
        errors
            .protocol(Some(path.as_str()), "source path does not match the authoritative file id");
        return None;
    }
    let mut import_end = 0;
    for import in &raw.imports {
        verify_import(import, raw.id, &path, sources, errors);
        ordered(import.span, &mut import_end, &path, errors, "import");
    }
    let mut type_owners = vec![0u32; raw.type_syntax.len()];
    let type_depths =
        verify_type_arena(&raw.type_syntax, raw.id, &path, sources, &mut type_owners, errors);
    let mut top_names = BTreeSet::new();
    let mut declaration_end = 0;
    let mut function_end = 0;
    let mut top_spans =
        Vec::with_capacity(raw.data_declarations.len().saturating_add(raw.functions.len()));
    for declaration in &raw.data_declarations {
        verify_declaration(
            declaration,
            raw.id,
            &path,
            sources,
            &mut type_owners,
            &mut top_names,
            errors,
        );
        ordered(declaration.span, &mut declaration_end, &path, errors, "data declaration");
        top_spans.push(declaration.span);
    }
    for function in &raw.functions {
        verify_function(function, raw.id, &path, sources, &mut type_owners, &mut top_names, errors);
        ordered(function.span, &mut function_end, &path, errors, "function");
        top_spans.push(function.span);
    }
    top_spans.sort_by_key(|span| (span.start, span.end));
    let mut merged_end = import_end;
    for span in top_spans {
        if span.start < merged_end {
            errors.node(&path, "top-level declaration overlaps an import or another declaration");
        }
        merged_end = merged_end.max(span.end);
    }
    if type_depths.iter().any(|depth| *depth > MAX_NESTING_DEPTH) {
        errors.limit("type syntax nesting exceeds the protocol-v4 limit");
    }
    if type_owners.iter().any(|owners| *owners != 1) {
        errors.node(&path, "type arena has a shared or orphan node");
    }
    verify_file_structure(&raw, &path, errors);
    Some(SourceUnit { id, path, raw })
}

fn ordered(
    raw: UntrustedSpan,
    previous_end: &mut u32,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    if raw.start < *previous_end {
        errors.node(path, format!("{label} is not in canonical source order"));
    }
    *previous_end = raw.end;
}
fn checked_span(
    raw: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) -> Option<Span> {
    if raw.file != file {
        errors.node(path, format!("{label} uses the wrong file id"));
        return None;
    }
    match sources.verify_span(raw) {
        Ok(span) => Some(span),
        Err(_) => {
            errors.node(path, format!("{label} span is invalid"));
            None
        }
    }
}
fn span_text(span: Span, sources: &SourceMap) -> Option<&str> {
    let resolved = sources.resolve(span).ok()?;
    let start = usize::try_from(span.start()).ok()?;
    let end = usize::try_from(span.end()).ok()?;
    resolved.source().text().get(start..end)
}
fn token(
    raw: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    expected: &str,
    label: &str,
) -> Option<Span> {
    let span = checked_span(raw, file, path, sources, errors, label)?;
    if span_text(span, sources) != Some(expected) {
        errors.node(path, format!("{label} spelling disagrees with authoritative source"));
    }
    Some(span)
}
fn identifier(
    raw: &RawIdentifierSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) {
    let Some(span) = checked_span(raw.span, file, path, sources, errors, label) else {
        return;
    };
    if !valid_identifier(&raw.text) || is_sensitive(&raw.text) {
        errors.node(path, format!("{label} is forbidden or exceeds its bound"));
    }
    if span_text(span, sources) != Some(raw.text.as_str()) {
        errors.node(path, format!("{label} spelling disagrees with authoritative source"));
    }
}
fn is_sensitive(text: &str) -> bool {
    matches!(text, "__proto__" | "prototype" | "constructor")
}
fn valid_identifier(text: &str) -> bool {
    if text.is_empty() || text.len() > 128 || !text.is_ascii() {
        return false;
    }
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
fn own_type(id: u32, owners: &mut [u32], path: &NormalizedSourcePath, errors: &mut Errors) {
    match usize::try_from(id).ok().and_then(|i| owners.get_mut(i)) {
        Some(owner) => *owner = owner.saturating_add(1),
        None => errors.node(path, "type root references an unknown arena node"),
    }
}

fn verify_import(
    raw: &RawImportSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
) {
    checked_span(raw.span, file, path, sources, errors, "import");
    token(raw.import_span, file, path, sources, errors, "import", "import keyword");
    token(raw.from_span, file, path, sources, errors, "from", "from keyword");
    token(raw.semicolon_span, file, path, sources, errors, ";", "import semicolon");
    let mut names = BTreeSet::new();
    let mut ordered_children = vec![raw.import_span];
    for binding in &raw.bindings {
        checked_span(binding.span, file, path, sources, errors, "import binding");
        identifier(&binding.imported, file, path, sources, errors, "imported name");
        identifier(&binding.local, file, path, sources, errors, "local import name");
        if !names.insert(binding.local.text.as_str()) {
            errors.node(path, "duplicate local import name");
        }
        match binding.as_span {
            Some(span) => {
                token(span, file, path, sources, errors, "as", "as keyword");
            }
            None if binding.imported != binding.local => {
                errors.node(path, "unaliased import must repeat the exact identifier")
            }
            None => {}
        }
        for child in [Some(binding.imported.span), binding.as_span, Some(binding.local.span)]
            .into_iter()
            .flatten()
        {
            require_claim_contains(binding.span, child, path, errors, "import binding child");
        }
        if let Some(as_span) = binding.as_span {
            require_claim_order(
                &[binding.imported.span, as_span, binding.local.span],
                path,
                errors,
                "import binding tokens",
            );
        }
        ordered_children.push(binding.span);
    }
    let spec = &raw.specifier;
    let Some(token_span) =
        checked_span(spec.token_span, file, path, sources, errors, "module specifier token")
    else {
        return;
    };
    let Some(value_span) =
        checked_span(spec.value_span, file, path, sources, errors, "module specifier value")
    else {
        return;
    };
    if span_text(value_span, sources) != Some(spec.text.as_str()) || !valid_specifier(&spec.text) {
        errors.node(path, "module specifier is not canonical explicit-relative .zry syntax");
    }
    let double = format!("\"{}\"", spec.text);
    let single = format!("'{}'", spec.text);
    if !span_text(token_span, sources).is_some_and(|text| text == double || text == single) {
        errors.node(path, "module specifier token disagrees with authoritative source");
    }
    require_claim_contains(raw.span, spec.token_span, path, errors, "module specifier token");
    require_claim_contains(
        spec.token_span,
        spec.value_span,
        path,
        errors,
        "module specifier value",
    );
    ordered_children.extend([raw.from_span, spec.token_span, raw.semicolon_span]);
    for child in &ordered_children {
        require_claim_contains(raw.span, *child, path, errors, "import child");
    }
    require_claim_order(&ordered_children, path, errors, "import children");
}
fn valid_specifier(text: &str) -> bool {
    (text.starts_with("./") || text.starts_with("../"))
        && text.ends_with(".zry")
        && text.len() <= 1024
        && text.is_ascii()
        && !text.contains("//")
        && !text.contains(['\\', '?', '#'])
        && !text.contains("://")
}

fn verify_type_arena(
    raw: &[RawTypeSyntax],
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    owners: &mut [u32],
    errors: &mut Errors,
) -> Vec<u32> {
    let mut depths = vec![1u32; raw.len()];
    for (index, node) in raw.iter().enumerate() {
        checked_span(node.span, file, path, sources, errors, "type syntax");
        match &node.kind {
            RawTypeSyntaxKind::Missing => {
                if node.span.start != node.span.end {
                    errors.node(path, "missing type node must use an empty insertion span");
                }
            }
            RawTypeSyntaxKind::Named { name } => {
                identifier(name, file, path, sources, errors, "named type")
            }
            RawTypeSyntaxKind::String { keyword_span } => {
                token(*keyword_span, file, path, sources, errors, "String", "String type");
            }
            RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } => type_container(
                index,
                "Vec",
                *keyword_span,
                *less_than_span,
                *argument,
                *greater_than_span,
                file,
                path,
                sources,
                owners,
                &mut depths,
                errors,
            ),
            RawTypeSyntaxKind::Shared {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } => type_container(
                index,
                "Shared",
                *keyword_span,
                *less_than_span,
                *argument,
                *greater_than_span,
                file,
                path,
                sources,
                owners,
                &mut depths,
                errors,
            ),
            RawTypeSyntaxKind::Weak {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } => type_container(
                index,
                "Weak",
                *keyword_span,
                *less_than_span,
                *argument,
                *greater_than_span,
                file,
                path,
                sources,
                owners,
                &mut depths,
                errors,
            ),
            RawTypeSyntaxKind::Borrow {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } => type_container(
                index,
                "Borrow",
                *keyword_span,
                *less_than_span,
                *argument,
                *greater_than_span,
                file,
                path,
                sources,
                owners,
                &mut depths,
                errors,
            ),
            RawTypeSyntaxKind::BorrowMut {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            } => type_container(
                index,
                "BorrowMut",
                *keyword_span,
                *less_than_span,
                *argument,
                *greater_than_span,
                file,
                path,
                sources,
                owners,
                &mut depths,
                errors,
            ),
            RawTypeSyntaxKind::FixedArray {
                keyword_span,
                less_than_span,
                element,
                comma_span,
                length_span,
                length_spelling,
                length,
                greater_than_span,
            } => {
                token(*keyword_span, file, path, sources, errors, "FixedArray", "FixedArray type");
                token(*less_than_span, file, path, sources, errors, "<", "type open angle");
                token(*comma_span, file, path, sources, errors, ",", "type comma");
                token(*greater_than_span, file, path, sources, errors, ">", "type close angle");
                let verified_length_span =
                    checked_span(*length_span, file, path, sources, errors, "fixed-array length");
                if !canonical_u32(length_spelling)
                    || length_spelling.parse::<u32>().ok() != Some(*length)
                    || *length > MAX_FIXED_ARRAY_LENGTH
                    || verified_length_span.and_then(|span| span_text(span, sources))
                        != Some(length_spelling.as_str())
                {
                    errors.node(
                        path,
                        "fixed-array length is not a canonical source-authenticated u32",
                    );
                }
                type_edge(*element, index, owners, &mut depths, path, errors);
            }
        }
        let child = match node.kind {
            RawTypeSyntaxKind::Vec { argument, .. }
            | RawTypeSyntaxKind::Shared { argument, .. }
            | RawTypeSyntaxKind::Weak { argument, .. }
            | RawTypeSyntaxKind::Borrow { argument, .. }
            | RawTypeSyntaxKind::BorrowMut { argument, .. } => Some(argument),
            RawTypeSyntaxKind::FixedArray { element, .. } => Some(element),
            _ => None,
        };
        if let Some(child) =
            child.and_then(|id| usize::try_from(id).ok()).and_then(|id| raw.get(id))
            && !contains_claim(node.span, child.span)
        {
            errors.node(path, "type child span is outside its parent type span");
        }
    }
    depths
}
#[allow(clippy::too_many_arguments)]
fn type_container(
    index: usize,
    expected: &str,
    keyword: UntrustedSpan,
    less: UntrustedSpan,
    argument: u32,
    greater: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    owners: &mut [u32],
    depths: &mut [u32],
    errors: &mut Errors,
) {
    token(keyword, file, path, sources, errors, expected, "container type keyword");
    token(less, file, path, sources, errors, "<", "type open angle");
    token(greater, file, path, sources, errors, ">", "type close angle");
    type_edge(argument, index, owners, depths, path, errors);
}
fn type_edge(
    raw: u32,
    parent: usize,
    owners: &mut [u32],
    depths: &mut [u32],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let Some(child) = usize::try_from(raw).ok() else {
        return;
    };
    if child >= parent {
        errors.node(path, "type edge is not canonical postorder");
        return;
    }
    let Some(owner) = owners.get_mut(child) else {
        errors.node(path, "type edge references an unknown node");
        return;
    };
    *owner = owner.saturating_add(1);
    depths[parent] = depths[parent].max(depths[child].saturating_add(1));
}
fn canonical_u32(value: &str) -> bool {
    value.len() <= 10
        && (value == "0" || (!value.starts_with('0') && value.bytes().all(|b| b.is_ascii_digit())))
        && value.parse::<u32>().is_ok_and(|length| length <= MAX_FIXED_ARRAY_LENGTH)
}

fn verify_declaration(
    raw: &RawDataDeclaration,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    type_owners: &mut [u32],
    top_names: &mut BTreeSet<String>,
    errors: &mut Errors,
) {
    checked_span(raw.span, file, path, sources, errors, "data declaration");
    if let Some(span) = raw.export_span {
        token(span, file, path, sources, errors, "export", "export keyword");
    }
    let (name, members): (&RawIdentifierSyntax, usize) = match &raw.kind {
        RawDataDeclarationKind::Struct {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            fields,
            close_brace_span,
        } => {
            token(*interface_span, file, path, sources, errors, "interface", "interface keyword");
            token(*extends_span, file, path, sources, errors, "extends", "extends keyword");
            token(*marker_span, file, path, sources, errors, "ZrynaStruct", "struct marker");
            token(*open_brace_span, file, path, sources, errors, "{", "declaration open brace");
            token(*close_brace_span, file, path, sources, errors, "}", "declaration close brace");
            let mut member_names = BTreeSet::new();
            for field in fields {
                checked_span(field.span, file, path, sources, errors, "struct field");
                identifier(&field.name, file, path, sources, errors, "field name");
                if !member_names.insert(field.name.text.as_str()) {
                    errors.node(path, "duplicate struct field name");
                }
                token(field.colon_span, file, path, sources, errors, ":", "field colon");
                token(field.semicolon_span, file, path, sources, errors, ";", "field semicolon");
                own_type(field.type_syntax, type_owners, path, errors);
            }
            (name, fields.len())
        }
        RawDataDeclarationKind::Enum {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            variants,
            close_brace_span,
        } => {
            token(*interface_span, file, path, sources, errors, "interface", "interface keyword");
            token(*extends_span, file, path, sources, errors, "extends", "extends keyword");
            token(*marker_span, file, path, sources, errors, "ZrynaEnum", "enum marker");
            token(*open_brace_span, file, path, sources, errors, "{", "declaration open brace");
            token(*close_brace_span, file, path, sources, errors, "}", "declaration close brace");
            let mut member_names = BTreeSet::new();
            for variant in variants {
                checked_span(variant.span, file, path, sources, errors, "enum variant");
                identifier(&variant.name, file, path, sources, errors, "variant name");
                if !member_names.insert(variant.name.text.as_str()) {
                    errors.node(path, "duplicate enum variant name");
                }
                token(variant.colon_span, file, path, sources, errors, ":", "variant colon");
                token(
                    variant.semicolon_span,
                    file,
                    path,
                    sources,
                    errors,
                    ";",
                    "variant semicolon",
                );
                match (variant.payload_type, variant.none_span) {
                    (Some(root), None) => own_type(root, type_owners, path, errors),
                    (None, Some(span)) => {
                        token(
                            span,
                            file,
                            path,
                            sources,
                            errors,
                            "ZrynaNone",
                            "payload-free marker",
                        );
                    }
                    _ => errors.node(path, "enum variant must contain exactly one payload form"),
                }
            }
            (name, variants.len())
        }
    };
    let _ = members;
    identifier(name, file, path, sources, errors, "data declaration name");
    if !contains_claim(raw.span, name.span) {
        errors.node(path, "data declaration name is outside its owner span");
    }
    if !top_names.insert(name.text.clone()) {
        errors.node(path, "duplicate top-level declaration name");
    }
}

fn verify_function(
    raw: &RawFunctionSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    type_owners: &mut [u32],
    top_names: &mut BTreeSet<String>,
    errors: &mut Errors,
) {
    checked_span(raw.span, file, path, sources, errors, "function");
    if let Some(span) = raw.export_span {
        token(span, file, path, sources, errors, "export", "export keyword");
    }
    token(raw.function_span, file, path, sources, errors, "function", "function keyword");
    identifier(&raw.name, file, path, sources, errors, "function name");
    if !contains_claim(raw.span, raw.function_span)
        || !contains_claim(raw.span, raw.name.span)
        || !contains_claim(raw.span, raw.body.span)
    {
        errors.node(path, "function child span is outside its owner span");
    }
    if !top_names.insert(raw.name.text.clone()) {
        errors.node(path, "duplicate top-level declaration name");
    }
    let mut locals = BTreeSet::new();
    for parameter in &raw.parameters {
        checked_span(parameter.span, file, path, sources, errors, "parameter");
        identifier(&parameter.name, file, path, sources, errors, "parameter name");
        if !locals.insert(parameter.name.text.clone()) {
            errors.node(path, "duplicate parameter name");
        }
        own_type(parameter.type_syntax, type_owners, path, errors);
    }
    own_type(raw.result_type, type_owners, path, errors);
    verify_body(&raw.body, file, path, sources, type_owners, &mut locals, errors);
}

fn verify_body(
    raw: &RawFunctionBodySyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    type_owners: &mut [u32],
    locals: &mut BTreeSet<String>,
    errors: &mut Errors,
) {
    checked_span(raw.span, file, path, sources, errors, "function body");
    if raw.root_block != 0 {
        errors.node(path, "root block id is not zero");
    }
    let mut block_owners = vec![0u32; raw.blocks.len()];
    if let Some(root) = block_owners.get_mut(0) {
        *root = 1;
    }
    let mut statement_owners = vec![0u32; raw.statements.len()];
    for block in &raw.blocks {
        checked_span(block.span, file, path, sources, errors, "block");
        token(block.open_brace_span, file, path, sources, errors, "{", "block open brace");
        token(block.close_brace_span, file, path, sources, errors, "}", "block close brace");
        for id in &block.statements {
            match usize::try_from(*id).ok().and_then(|i| statement_owners.get_mut(i)) {
                Some(owner) => *owner = owner.saturating_add(1),
                None => errors.node(path, "block references an unknown statement"),
            }
        }
    }
    let mut expression_owners = vec![0u32; raw.expressions.len()];
    let mut expression_depths = vec![1u32; raw.expressions.len()];
    for (index, expression) in raw.expressions.iter().enumerate() {
        verify_expression(
            expression,
            index,
            file,
            path,
            sources,
            type_owners,
            &mut expression_owners,
            &mut expression_depths,
            errors,
        );
    }
    for expression in &raw.expressions {
        match &expression.kind {
            RawExpressionKind::Borrow { value, .. }
            | RawExpressionKind::BorrowMut { value, .. }
                if !is_place(&raw.expressions, *value) =>
            {
                errors.node(path, "borrow operand is not syntactically a place")
            }
            RawExpressionKind::VecPush { vector, .. } if !is_place(&raw.expressions, *vector) => {
                errors.node(path, "push target is not syntactically a place")
            }
            _ => {}
        }
    }
    for statement in &raw.statements {
        verify_statement(
            statement,
            file,
            path,
            sources,
            type_owners,
            locals,
            raw.expressions.len(),
            &mut expression_owners,
            &mut block_owners,
            errors,
        );
    }
    for block in &raw.blocks {
        if !contains_claim(raw.span, block.span) {
            errors.node(path, "block span is outside its function body");
        }
        for statement in &block.statements {
            if let Some(statement) =
                usize::try_from(*statement).ok().and_then(|id| raw.statements.get(id))
                && !contains_claim(block.span, statement.span)
            {
                errors.node(path, "statement span is outside its owning block");
            }
        }
    }
    for expression in &raw.expressions {
        if !contains_claim(raw.span, expression.span) {
            errors.node(path, "expression span is outside its function body");
        }
        for child in expression_children(&expression.kind) {
            if let Some(child) = usize::try_from(child).ok().and_then(|id| raw.expressions.get(id))
                && !contains_claim(expression.span, child.span)
            {
                errors.node(path, "expression child span is outside its parent expression");
            }
        }
    }
    for statement in &raw.statements {
        for root in statement_expression_roots(&statement.kind) {
            if let Some(root) = usize::try_from(root).ok().and_then(|id| raw.expressions.get(id))
                && !contains_claim(statement.span, root.span)
            {
                errors.node(path, "root expression span is outside its owning statement");
            }
        }
    }
    for statement in &raw.statements {
        if let RawStatementKind::Assignment { target, .. } = statement.kind {
            if !is_place(&raw.expressions, target) {
                errors.node(path, "assignment target is not syntactically a place");
            }
        }
    }
    if block_owners.iter().any(|owners| *owners != 1) {
        errors.node(path, "block arena has a shared or orphan block");
    }
    if statement_owners.iter().any(|owners| *owners != 1) {
        errors.node(path, "statement arena has a shared or orphan statement");
    }
    if expression_owners.iter().any(|owners| *owners != 1) {
        errors.node(path, "expression arena has a shared or orphan expression");
    }
    if expression_depths.iter().any(|depth| *depth > MAX_NESTING_DEPTH) {
        errors.limit("expression nesting exceeds the protocol-v4 limit");
    }
    verify_arena_order(raw, path, errors);
}
fn contains_claim(parent: UntrustedSpan, child: UntrustedSpan) -> bool {
    parent.file == child.file && child.start >= parent.start && child.end <= parent.end
}
fn require_claim_contains(
    parent: UntrustedSpan,
    child: UntrustedSpan,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    if !contains_claim(parent, child) {
        errors.node(path, format!("{label} is outside its owner span"));
    }
}
fn require_claim_order(
    spans: &[UntrustedSpan],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    for pair in spans.windows(2) {
        if pair[0].file != pair[1].file || pair[1].start < pair[0].end {
            errors.node(path, format!("{label} are not in source order"));
        }
    }
}
fn statement_expression_roots(kind: &RawStatementKind) -> Vec<u32> {
    match kind {
        RawStatementKind::LocalDeclaration { initializer, .. } => vec![*initializer],
        RawStatementKind::Assignment { target, value, .. } => vec![*target, *value],
        RawStatementKind::Return { value, .. } => vec![*value],
        RawStatementKind::If { condition, .. } | RawStatementKind::While { condition, .. } => {
            vec![*condition]
        }
        RawStatementKind::ExpressionStatement { expression, .. } => vec![*expression],
        RawStatementKind::WeakUpgrade { weak, .. } => vec![*weak],
        RawStatementKind::Block { .. } => Vec::new(),
    }
}
fn expression_children(kind: &RawExpressionKind) -> Vec<u32> {
    match kind {
        RawExpressionKind::Negation { operand, .. } => vec![*operand],
        RawExpressionKind::Addition { lhs, rhs, .. }
        | RawExpressionKind::Subtraction { lhs, rhs, .. }
        | RawExpressionKind::Multiplication { lhs, rhs, .. }
        | RawExpressionKind::Equal { lhs, rhs, .. }
        | RawExpressionKind::NotEqual { lhs, rhs, .. }
        | RawExpressionKind::LessThan { lhs, rhs, .. }
        | RawExpressionKind::LessEqual { lhs, rhs, .. }
        | RawExpressionKind::GreaterThan { lhs, rhs, .. }
        | RawExpressionKind::GreaterEqual { lhs, rhs, .. } => vec![*lhs, *rhs],
        RawExpressionKind::Call { arguments, .. } => arguments.clone(),
        RawExpressionKind::StructConstruction { fields, .. } => fields
            .iter()
            .map(|field| match field.kind {
                RawFieldInitializerKind::Explicit { value, .. }
                | RawFieldInitializerKind::Shorthand { value, .. } => value,
            })
            .collect(),
        RawExpressionKind::EnumConstruction { payload, .. } => payload.iter().copied().collect(),
        RawExpressionKind::FixedArrayConstruction { elements, .. }
        | RawExpressionKind::VecConstruction { elements, .. } => elements.clone(),
        RawExpressionKind::FieldAccess { base, .. } => vec![*base],
        RawExpressionKind::Index { base, index, .. } => vec![*base, *index],
        RawExpressionKind::Clone { value, .. }
        | RawExpressionKind::Shared { value, .. }
        | RawExpressionKind::Downgrade { value, .. }
        | RawExpressionKind::Borrow { value, .. }
        | RawExpressionKind::BorrowMut { value, .. } => vec![*value],
        RawExpressionKind::VecPush { vector, value, .. } => vec![*vector, *value],
        RawExpressionKind::Match { scrutinee, arms, .. } => {
            std::iter::once(*scrutinee).chain(arms.iter().map(|arm| arm.value)).collect()
        }
        RawExpressionKind::Reference { .. }
        | RawExpressionKind::BoolLiteral { .. }
        | RawExpressionKind::I32Literal { .. }
        | RawExpressionKind::StringLiteral { .. } => Vec::new(),
    }
}

fn verify_file_structure(raw: &RawSourceUnit, path: &NormalizedSourcePath, errors: &mut Errors) {
    for (index, node) in raw.type_syntax.iter().enumerate() {
        verify_type_structure(node, index, &raw.type_syntax, path, errors);
    }
    for declaration in &raw.data_declarations {
        verify_declaration_structure(declaration, &raw.type_syntax, path, errors);
    }
    for function in &raw.functions {
        verify_function_structure(function, &raw.type_syntax, path, errors);
    }
}
fn checked_index<T>(values: &[T], id: u32) -> Option<&T> {
    usize::try_from(id).ok().and_then(|index| values.get(index))
}
fn check_sequence(
    owner: UntrustedSpan,
    children: &[UntrustedSpan],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    for child in children {
        require_claim_contains(owner, *child, path, errors, label);
    }
    require_claim_order(children, path, errors, label);
}
fn verify_type_structure(
    node: &RawTypeSyntax,
    _: usize,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let children = match &node.kind {
        RawTypeSyntaxKind::Missing => Vec::new(),
        RawTypeSyntaxKind::Named { name } => vec![name.span],
        RawTypeSyntaxKind::String { keyword_span } => vec![*keyword_span],
        RawTypeSyntaxKind::Vec { keyword_span, less_than_span, argument, greater_than_span }
        | RawTypeSyntaxKind::Shared { keyword_span, less_than_span, argument, greater_than_span }
        | RawTypeSyntaxKind::Weak { keyword_span, less_than_span, argument, greater_than_span }
        | RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
        | RawTypeSyntaxKind::BorrowMut {
            keyword_span,
            less_than_span,
            argument,
            greater_than_span,
        } => checked_index(types, *argument).map_or_else(Vec::new, |argument| {
            vec![*keyword_span, *less_than_span, argument.span, *greater_than_span]
        }),
        RawTypeSyntaxKind::FixedArray {
            keyword_span,
            less_than_span,
            element,
            comma_span,
            length_span,
            greater_than_span,
            ..
        } => checked_index(types, *element).map_or_else(Vec::new, |element| {
            vec![
                *keyword_span,
                *less_than_span,
                element.span,
                *comma_span,
                *length_span,
                *greater_than_span,
            ]
        }),
    };
    check_sequence(node.span, &children, path, errors, "type syntax children");
}
fn verify_declaration_structure(
    raw: &RawDataDeclaration,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let mut children = raw.export_span.into_iter().collect::<Vec<_>>();
    match &raw.kind {
        RawDataDeclarationKind::Struct {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            fields,
            close_brace_span,
        } => {
            children.extend([
                *interface_span,
                name.span,
                *extends_span,
                *marker_span,
                *open_brace_span,
            ]);
            for field in fields {
                let mut member = vec![field.name.span, field.colon_span];
                if let Some(ty) = checked_index(types, field.type_syntax) {
                    member.push(ty.span);
                }
                member.push(field.semicolon_span);
                check_sequence(field.span, &member, path, errors, "struct field children");
                children.push(field.span);
            }
            children.push(*close_brace_span);
        }
        RawDataDeclarationKind::Enum {
            interface_span,
            name,
            extends_span,
            marker_span,
            open_brace_span,
            variants,
            close_brace_span,
        } => {
            children.extend([
                *interface_span,
                name.span,
                *extends_span,
                *marker_span,
                *open_brace_span,
            ]);
            for variant in variants {
                let mut member = vec![variant.name.span, variant.colon_span];
                if let Some(id) = variant.payload_type
                    && let Some(ty) = checked_index(types, id)
                {
                    member.push(ty.span);
                }
                if let Some(span) = variant.none_span {
                    member.push(span);
                }
                member.push(variant.semicolon_span);
                check_sequence(variant.span, &member, path, errors, "enum variant children");
                children.push(variant.span);
            }
            children.push(*close_brace_span);
        }
    }
    check_sequence(raw.span, &children, path, errors, "data declaration children");
}
fn verify_function_structure(
    raw: &RawFunctionSyntax,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let mut children = raw.export_span.into_iter().collect::<Vec<_>>();
    children.extend([raw.function_span, raw.name.span]);
    for parameter in &raw.parameters {
        let mut inner = vec![parameter.name.span];
        if let Some(ty) = checked_index(types, parameter.type_syntax) {
            inner.push(ty.span);
        }
        check_sequence(parameter.span, &inner, path, errors, "parameter children");
        children.push(parameter.span);
    }
    if let Some(result) = checked_index(types, raw.result_type) {
        children.push(result.span);
    }
    children.push(raw.body.span);
    check_sequence(raw.span, &children, path, errors, "function children");
    verify_body_structure(&raw.body, types, path, errors);
}
fn verify_body_structure(
    raw: &RawFunctionBodySyntax,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    for block in &raw.blocks {
        let mut children = vec![block.open_brace_span];
        for statement in &block.statements {
            if let Some(statement) = checked_index(&raw.statements, *statement) {
                children.push(statement.span);
            }
        }
        children.push(block.close_brace_span);
        check_sequence(block.span, &children, path, errors, "block children");
    }
    for statement in &raw.statements {
        verify_statement_structure(statement, raw, types, path, errors);
    }
    for expression in &raw.expressions {
        verify_expression_structure(expression, raw, types, path, errors);
    }
}
fn expression_span(body: &RawFunctionBodySyntax, id: u32) -> Option<UntrustedSpan> {
    checked_index(&body.expressions, id).map(|value| value.span)
}
fn block_span(body: &RawFunctionBodySyntax, id: u32) -> Option<UntrustedSpan> {
    checked_index(&body.blocks, id).map(|value| value.span)
}
fn verify_statement_structure(
    raw: &RawStatementSyntax,
    body: &RawFunctionBodySyntax,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let children = match &raw.kind {
        RawStatementKind::LocalDeclaration {
            keyword_span,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
            ..
        } => {
            let mut out = vec![*keyword_span, name.span];
            if let Some(ty) = checked_index(types, *type_syntax) {
                out.push(ty.span);
            }
            out.push(*equals_span);
            if let Some(span) = expression_span(body, *initializer) {
                out.push(span);
            }
            out.push(*semicolon_span);
            out
        }
        RawStatementKind::Assignment { target, equals_span, value, semicolon_span } => [
            expression_span(body, *target),
            Some(*equals_span),
            expression_span(body, *value),
            Some(*semicolon_span),
        ]
        .into_iter()
        .flatten()
        .collect(),
        RawStatementKind::Return { keyword_span, value, semicolon_span } => {
            [Some(*keyword_span), expression_span(body, *value), Some(*semicolon_span)]
                .into_iter()
                .flatten()
                .collect()
        }
        RawStatementKind::Block { block } => block_span(body, *block).into_iter().collect(),
        RawStatementKind::If {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            then_block,
            else_clause,
        } => {
            let mut out = [
                Some(*keyword_span),
                Some(*open_paren_span),
                expression_span(body, *condition),
                Some(*close_paren_span),
                block_span(body, *then_block),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            if let Some(value) = else_clause {
                out.push(value.keyword_span);
                if let Some(span) = block_span(body, value.block) {
                    out.push(span);
                }
            }
            out
        }
        RawStatementKind::While {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            body_block,
        } => [
            Some(*keyword_span),
            Some(*open_paren_span),
            expression_span(body, *condition),
            Some(*close_paren_span),
            block_span(body, *body_block),
        ]
        .into_iter()
        .flatten()
        .collect(),
        RawStatementKind::ExpressionStatement { expression, semicolon_span } => {
            [expression_span(body, *expression), Some(*semicolon_span)]
                .into_iter()
                .flatten()
                .collect()
        }
        RawStatementKind::WeakUpgrade {
            keyword_span,
            weak,
            binding,
            as_span,
            success_block,
            else_span,
            failure_block,
        } => [
            Some(*keyword_span),
            expression_span(body, *weak),
            Some(binding.span),
            Some(*as_span),
            block_span(body, *success_block),
            Some(*else_span),
            block_span(body, *failure_block),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    check_sequence(raw.span, &children, path, errors, "statement children");
}
fn verify_expression_structure(
    raw: &RawExpressionSyntax,
    body: &RawFunctionBodySyntax,
    types: &[RawTypeSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let child = |id| expression_span(body, id);
    let children = match &raw.kind {
        RawExpressionKind::Reference { name } => vec![name.span],
        RawExpressionKind::BoolLiteral { .. }
        | RawExpressionKind::I32Literal { .. }
        | RawExpressionKind::StringLiteral { .. } => Vec::new(),
        RawExpressionKind::Negation { operator_span, operand } => {
            [Some(*operator_span), child(*operand)].into_iter().flatten().collect()
        }
        RawExpressionKind::Addition { operator_span, lhs, rhs }
        | RawExpressionKind::Subtraction { operator_span, lhs, rhs }
        | RawExpressionKind::Multiplication { operator_span, lhs, rhs }
        | RawExpressionKind::Equal { operator_span, lhs, rhs }
        | RawExpressionKind::NotEqual { operator_span, lhs, rhs }
        | RawExpressionKind::LessThan { operator_span, lhs, rhs }
        | RawExpressionKind::LessEqual { operator_span, lhs, rhs }
        | RawExpressionKind::GreaterThan { operator_span, lhs, rhs }
        | RawExpressionKind::GreaterEqual { operator_span, lhs, rhs } => {
            [child(*lhs), Some(*operator_span), child(*rhs)].into_iter().flatten().collect()
        }
        RawExpressionKind::Call { callee, open_paren_span, arguments, close_paren_span } => {
            let mut out = vec![callee.span, *open_paren_span];
            out.extend(arguments.iter().filter_map(|id| child(*id)));
            out.push(*close_paren_span);
            out
        }
        RawExpressionKind::StructConstruction {
            type_name,
            open_paren_span,
            open_brace_span,
            fields,
            close_brace_span,
            close_paren_span,
        } => {
            let mut out = vec![type_name.span, *open_paren_span, *open_brace_span];
            for field in fields {
                match &field.kind {
                    RawFieldInitializerKind::Shorthand { name, value } => {
                        require_claim_contains(
                            field.span,
                            name.span,
                            path,
                            errors,
                            "shorthand initializer name",
                        );
                        if let Some(value) = child(*value) {
                            require_claim_contains(
                                field.span,
                                value,
                                path,
                                errors,
                                "shorthand initializer value",
                            );
                        }
                    }
                    RawFieldInitializerKind::Explicit { name, colon_span, value } => {
                        let inner = [Some(name.span), Some(*colon_span), child(*value)]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                        check_sequence(
                            field.span,
                            &inner,
                            path,
                            errors,
                            "explicit initializer children",
                        );
                    }
                }
                out.push(field.span);
            }
            out.extend([*close_brace_span, *close_paren_span]);
            out
        }
        RawExpressionKind::EnumConstruction {
            type_name,
            dot_span,
            variant,
            open_paren_span,
            payload,
            close_paren_span,
        } => {
            let mut out = vec![type_name.span, *dot_span, variant.span, *open_paren_span];
            out.extend(payload.iter().filter_map(|id| child(*id)));
            out.push(*close_paren_span);
            out
        }
        RawExpressionKind::FixedArrayConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
            ..
        }
        | RawExpressionKind::VecConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
            ..
        } => {
            let mut out =
                checked_index(types, *type_syntax).map_or_else(Vec::new, |ty| vec![ty.span]);
            out.extend([*open_paren_span, *open_bracket_span]);
            out.extend(elements.iter().filter_map(|id| child(*id)));
            out.extend([*close_bracket_span, *close_paren_span]);
            out
        }
        RawExpressionKind::FieldAccess { base, dot_span, field } => {
            [child(*base), Some(*dot_span), Some(field.span)].into_iter().flatten().collect()
        }
        RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span } => {
            [child(*base), Some(*open_bracket_span), child(*index), Some(*close_bracket_span)]
                .into_iter()
                .flatten()
                .collect()
        }
        RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span }
        | RawExpressionKind::Shared { keyword_span, open_paren_span, value, close_paren_span }
        | RawExpressionKind::Downgrade { keyword_span, open_paren_span, value, close_paren_span }
        | RawExpressionKind::Borrow { keyword_span, open_paren_span, value, close_paren_span }
        | RawExpressionKind::BorrowMut { keyword_span, open_paren_span, value, close_paren_span } => {
            [Some(*keyword_span), Some(*open_paren_span), child(*value), Some(*close_paren_span)]
                .into_iter()
                .flatten()
                .collect()
        }
        RawExpressionKind::VecPush {
            keyword_span,
            open_paren_span,
            vector,
            comma_span,
            value,
            close_paren_span,
        } => [
            Some(*keyword_span),
            Some(*open_paren_span),
            child(*vector),
            Some(*comma_span),
            child(*value),
            Some(*close_paren_span),
        ]
        .into_iter()
        .flatten()
        .collect(),
        RawExpressionKind::Match {
            keyword_span,
            open_paren_span,
            scrutinee,
            close_paren_span,
            open_brace_span,
            arms,
            close_brace_span,
        } => {
            let mut out = [
                Some(*keyword_span),
                Some(*open_paren_span),
                child(*scrutinee),
                Some(*open_brace_span),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            for arm in arms {
                let mut inner = vec![arm.type_name.span, arm.dot_span, arm.variant.span];
                if let Some(binding) = &arm.binding {
                    inner.push(binding.span);
                }
                inner.push(arm.arrow_span);
                if let Some(value) = child(arm.value) {
                    inner.push(value);
                }
                check_sequence(arm.span, &inner, path, errors, "match arm children");
                out.push(arm.span);
            }
            out.push(*close_brace_span);
            out.push(*close_paren_span);
            out
        }
    };
    check_sequence(raw.span, &children, path, errors, "expression children");
}
fn is_place(expressions: &[RawExpressionSyntax], id: u32) -> bool {
    let mut current = id;
    for _ in 0..=MAX_NESTING_DEPTH {
        let Some(expression) =
            usize::try_from(current).ok().and_then(|index| expressions.get(index))
        else {
            return false;
        };
        match &expression.kind {
            RawExpressionKind::Reference { .. } => return true,
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. } => {
                current = *base
            }
            _ => return false,
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn verify_statement(
    raw: &RawStatementSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    type_owners: &mut [u32],
    locals: &mut BTreeSet<String>,
    expression_count: usize,
    expression_owners: &mut [u32],
    block_owners: &mut [u32],
    errors: &mut Errors,
) {
    checked_span(raw.span, file, path, sources, errors, "statement");
    let mut bad_expression = false;
    let mut bad_block = false;
    let mut expression = |id| {
        if !own_expression_root(id, expression_count, expression_owners) {
            bad_expression = true;
        }
    };
    let mut block = |id| {
        if !own_block(id, block_owners) {
            bad_block = true;
        }
    };
    match &raw.kind {
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
        } => {
            token(
                *keyword_span,
                file,
                path,
                sources,
                errors,
                if *mutable { "let" } else { "const" },
                "local keyword",
            );
            identifier(name, file, path, sources, errors, "local name");
            if !locals.insert(name.text.clone()) {
                errors.node(path, "duplicate function-local binding name");
            }
            own_type(*type_syntax, type_owners, path, errors);
            token(*equals_span, file, path, sources, errors, "=", "initializer equals");
            expression(*initializer);
            token(*semicolon_span, file, path, sources, errors, ";", "local semicolon");
        }
        RawStatementKind::Assignment { target, equals_span, value, semicolon_span } => {
            expression(*target);
            token(*equals_span, file, path, sources, errors, "=", "assignment equals");
            expression(*value);
            token(*semicolon_span, file, path, sources, errors, ";", "assignment semicolon");
        }
        RawStatementKind::Return { keyword_span, value, semicolon_span } => {
            token(*keyword_span, file, path, sources, errors, "return", "return keyword");
            expression(*value);
            token(*semicolon_span, file, path, sources, errors, ";", "return semicolon");
        }
        RawStatementKind::Block { block: id } => block(*id),
        RawStatementKind::If {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            then_block,
            else_clause,
        } => {
            token(*keyword_span, file, path, sources, errors, "if", "if keyword");
            token(*open_paren_span, file, path, sources, errors, "(", "if open parenthesis");
            expression(*condition);
            token(*close_paren_span, file, path, sources, errors, ")", "if close parenthesis");
            block(*then_block);
            if let Some(value) = else_clause {
                token(value.keyword_span, file, path, sources, errors, "else", "else keyword");
                block(value.block);
            }
        }
        RawStatementKind::While {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            body_block,
        } => {
            token(*keyword_span, file, path, sources, errors, "while", "while keyword");
            token(*open_paren_span, file, path, sources, errors, "(", "while open parenthesis");
            expression(*condition);
            token(*close_paren_span, file, path, sources, errors, ")", "while close parenthesis");
            block(*body_block);
        }
        RawStatementKind::ExpressionStatement { expression: id, semicolon_span } => {
            expression(*id);
            token(*semicolon_span, file, path, sources, errors, ";", "expression semicolon");
        }
        RawStatementKind::WeakUpgrade {
            keyword_span,
            weak,
            as_span,
            binding,
            success_block,
            else_span,
            failure_block,
        } => {
            token(
                *keyword_span,
                file,
                path,
                sources,
                errors,
                "upgradeWeak",
                "weak-upgrade keyword",
            );
            expression(*weak);
            token(*as_span, file, path, sources, errors, "=>", "success arrow");
            identifier(binding, file, path, sources, errors, "weak-upgrade binding");
            if !locals.insert(binding.text.clone()) {
                errors.node(path, "duplicate weak-upgrade binding name");
            }
            block(*success_block);
            token(*else_span, file, path, sources, errors, "=>", "failure arrow");
            block(*failure_block);
        }
    }
    drop(expression);
    drop(block);
    if bad_expression {
        errors.node(path, "statement references an unknown expression");
    }
    if bad_block {
        errors.node(path, "statement references an unknown block");
    }
}
fn own_expression_root(id: u32, count: usize, owners: &mut [u32]) -> bool {
    let Some(index) = usize::try_from(id).ok() else {
        return false;
    };
    if index >= count {
        false
    } else {
        owners[index] = owners[index].saturating_add(1);
        true
    }
}
fn own_block(id: u32, owners: &mut [u32]) -> bool {
    match usize::try_from(id).ok().and_then(|i| owners.get_mut(i)) {
        Some(owner) => {
            *owner = owner.saturating_add(1);
            true
        }
        None => false,
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_expression(
    raw: &RawExpressionSyntax,
    index: usize,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    type_owners: &mut [u32],
    owners: &mut [u32],
    depths: &mut [u32],
    errors: &mut Errors,
) {
    let span = checked_span(raw.span, file, path, sources, errors, "expression");
    let mut edge_failures = Vec::new();
    let mut edge = |id| {
        if let Err(message) = expression_edge(id, index, owners, depths) {
            edge_failures.push(message);
        }
    };
    match &raw.kind {
        RawExpressionKind::Reference { name } => {
            identifier(name, file, path, sources, errors, "reference")
        }
        RawExpressionKind::BoolLiteral { value } => {
            if span.and_then(|s| span_text(s, sources))
                != Some(if *value { "true" } else { "false" })
            {
                errors.node(path, "Boolean literal disagrees with authoritative source");
            }
        }
        RawExpressionKind::I32Literal { spelling } => {
            if !canonical_i32(spelling)
                || span.and_then(|s| span_text(s, sources)) != Some(spelling.as_str())
            {
                errors.node(path, "i32 literal is not canonical or source faithful");
            }
        }
        RawExpressionKind::StringLiteral { spelling } => {
            if !canonical_string_literal(spelling)
                || span.and_then(|s| span_text(s, sources)) != Some(spelling.as_str())
            {
                errors.node(path, "string literal is not canonical or source faithful");
            }
        }
        RawExpressionKind::Negation { operator_span, operand } => {
            token(*operator_span, file, path, sources, errors, "-", "negation operator");
            edge(*operand);
        }
        RawExpressionKind::Addition { operator_span, lhs, rhs } => {
            binary(*operator_span, "+", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::Subtraction { operator_span, lhs, rhs } => {
            binary(*operator_span, "-", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::Multiplication { operator_span, lhs, rhs } => {
            binary(*operator_span, "*", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::Equal { operator_span, lhs, rhs } => {
            binary(*operator_span, "===", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::NotEqual { operator_span, lhs, rhs } => {
            binary(*operator_span, "!==", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::LessThan { operator_span, lhs, rhs } => {
            binary(*operator_span, "<", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::LessEqual { operator_span, lhs, rhs } => {
            binary(*operator_span, "<=", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::GreaterThan { operator_span, lhs, rhs } => {
            binary(*operator_span, ">", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::GreaterEqual { operator_span, lhs, rhs } => {
            binary(*operator_span, ">=", *lhs, *rhs, file, path, sources, &mut edge, errors)
        }
        RawExpressionKind::Call { callee, open_paren_span, arguments, close_paren_span } => {
            identifier(callee, file, path, sources, errors, "call callee");
            token(*open_paren_span, file, path, sources, errors, "(", "call open parenthesis");
            for id in arguments {
                edge(*id);
            }
            token(*close_paren_span, file, path, sources, errors, ")", "call close parenthesis");
        }
        RawExpressionKind::StructConstruction {
            type_name,
            open_paren_span,
            open_brace_span,
            fields,
            close_brace_span,
            close_paren_span,
        } => {
            identifier(type_name, file, path, sources, errors, "struct construction type");
            token(
                *open_paren_span,
                file,
                path,
                sources,
                errors,
                "(",
                "construction open parenthesis",
            );
            token(*open_brace_span, file, path, sources, errors, "{", "construction open brace");
            let mut names = BTreeSet::new();
            for field in fields {
                checked_span(field.span, file, path, sources, errors, "field initializer");
                match &field.kind {
                    RawFieldInitializerKind::Shorthand { name, value } => {
                        identifier(name, file, path, sources, errors, "initializer name");
                        if !names.insert(name.text.as_str()) {
                            errors.node(path, "duplicate initializer name");
                        }
                        edge(*value);
                    }
                    RawFieldInitializerKind::Explicit { name, colon_span, value } => {
                        identifier(name, file, path, sources, errors, "initializer name");
                        if !names.insert(name.text.as_str()) {
                            errors.node(path, "duplicate initializer name");
                        }
                        token(*colon_span, file, path, sources, errors, ":", "initializer colon");
                        edge(*value);
                    }
                }
            }
            token(*close_brace_span, file, path, sources, errors, "}", "construction close brace");
            token(
                *close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "construction close parenthesis",
            );
        }
        RawExpressionKind::EnumConstruction {
            type_name,
            dot_span,
            variant,
            open_paren_span,
            payload,
            close_paren_span,
        } => {
            identifier(type_name, file, path, sources, errors, "enum construction type");
            token(*dot_span, file, path, sources, errors, ".", "enum construction dot");
            identifier(variant, file, path, sources, errors, "enum construction variant");
            token(
                *open_paren_span,
                file,
                path,
                sources,
                errors,
                "(",
                "construction open parenthesis",
            );
            if let Some(id) = payload {
                edge(*id);
            }
            token(
                *close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "construction close parenthesis",
            );
        }
        RawExpressionKind::FixedArrayConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        }
        | RawExpressionKind::VecConstruction {
            type_syntax,
            open_paren_span,
            open_bracket_span,
            elements,
            close_bracket_span,
            close_paren_span,
        } => {
            own_type(*type_syntax, type_owners, path, errors);
            token(
                *open_paren_span,
                file,
                path,
                sources,
                errors,
                "(",
                "typed construction open parenthesis",
            );
            token(*open_bracket_span, file, path, sources, errors, "[", "array open bracket");
            for id in elements {
                edge(*id);
            }
            token(*close_bracket_span, file, path, sources, errors, "]", "array close bracket");
            token(
                *close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "typed construction close parenthesis",
            );
        }
        RawExpressionKind::FieldAccess { base, dot_span, field } => {
            edge(*base);
            token(*dot_span, file, path, sources, errors, ".", "field access dot");
            identifier(field, file, path, sources, errors, "field access name");
        }
        RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span } => {
            edge(*base);
            token(*open_bracket_span, file, path, sources, errors, "[", "index open bracket");
            edge(*index);
            token(*close_bracket_span, file, path, sources, errors, "]", "index close bracket");
        }
        RawExpressionKind::Clone { keyword_span, open_paren_span, value, close_paren_span } => {
            unary(
                "clone",
                *keyword_span,
                *open_paren_span,
                *value,
                *close_paren_span,
                file,
                path,
                sources,
                &mut edge,
                errors,
            )
        }
        RawExpressionKind::Shared { keyword_span, open_paren_span, value, close_paren_span } => {
            unary(
                "shared",
                *keyword_span,
                *open_paren_span,
                *value,
                *close_paren_span,
                file,
                path,
                sources,
                &mut edge,
                errors,
            )
        }
        RawExpressionKind::Downgrade { keyword_span, open_paren_span, value, close_paren_span } => {
            unary(
                "downgrade",
                *keyword_span,
                *open_paren_span,
                *value,
                *close_paren_span,
                file,
                path,
                sources,
                &mut edge,
                errors,
            )
        }
        RawExpressionKind::Borrow { keyword_span, open_paren_span, value, close_paren_span } => {
            unary(
                "borrow",
                *keyword_span,
                *open_paren_span,
                *value,
                *close_paren_span,
                file,
                path,
                sources,
                &mut edge,
                errors,
            )
        }
        RawExpressionKind::BorrowMut { keyword_span, open_paren_span, value, close_paren_span } => {
            unary(
                "borrowMut",
                *keyword_span,
                *open_paren_span,
                *value,
                *close_paren_span,
                file,
                path,
                sources,
                &mut edge,
                errors,
            )
        }
        RawExpressionKind::VecPush {
            keyword_span,
            open_paren_span,
            vector,
            comma_span,
            value,
            close_paren_span,
        } => {
            token(*keyword_span, file, path, sources, errors, "push", "push keyword");
            token(*open_paren_span, file, path, sources, errors, "(", "push open parenthesis");
            edge(*vector);
            token(*comma_span, file, path, sources, errors, ",", "push comma");
            edge(*value);
            token(*close_paren_span, file, path, sources, errors, ")", "push close parenthesis");
        }
        RawExpressionKind::Match {
            keyword_span,
            open_paren_span,
            scrutinee,
            close_paren_span,
            open_brace_span,
            arms,
            close_brace_span,
        } => {
            token(*keyword_span, file, path, sources, errors, "match", "match keyword");
            token(*open_paren_span, file, path, sources, errors, "(", "match open parenthesis");
            edge(*scrutinee);
            token(*open_brace_span, file, path, sources, errors, "{", "match open brace");
            let mut seen_arms = BTreeSet::new();
            for arm in arms {
                checked_span(arm.span, file, path, sources, errors, "match arm");
                identifier(&arm.type_name, file, path, sources, errors, "match arm type");
                token(arm.dot_span, file, path, sources, errors, ".", "match arm dot");
                identifier(&arm.variant, file, path, sources, errors, "match arm variant");
                if !seen_arms.insert((arm.type_name.text.as_str(), arm.variant.text.as_str())) {
                    errors.node(path, "duplicate qualified match arm");
                }
                if let Some(binding) = &arm.binding {
                    identifier(binding, file, path, sources, errors, "match arm binding");
                }
                token(arm.arrow_span, file, path, sources, errors, "=>", "match arm arrow");
                edge(arm.value);
            }
            token(*close_brace_span, file, path, sources, errors, "}", "match close brace");
            token(*close_paren_span, file, path, sources, errors, ")", "match close parenthesis");
        }
    }
    drop(edge);
    for message in edge_failures {
        errors.node(path, message);
    }
}

#[allow(clippy::too_many_arguments)]
fn binary<F: FnMut(u32)>(
    operator: UntrustedSpan,
    expected: &str,
    lhs: u32,
    rhs: u32,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    edge: &mut F,
    errors: &mut Errors,
) {
    token(operator, file, path, sources, errors, expected, "binary operator");
    edge(lhs);
    edge(rhs);
}
#[allow(clippy::too_many_arguments)]
fn unary<F: FnMut(u32)>(
    expected: &str,
    keyword: UntrustedSpan,
    open: UntrustedSpan,
    value: u32,
    close: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    edge: &mut F,
    errors: &mut Errors,
) {
    token(keyword, file, path, sources, errors, expected, "ownership intrinsic");
    token(open, file, path, sources, errors, "(", "intrinsic open parenthesis");
    edge(value);
    token(close, file, path, sources, errors, ")", "intrinsic close parenthesis");
}
fn expression_edge(
    raw: u32,
    parent: usize,
    owners: &mut [u32],
    depths: &mut [u32],
) -> Result<(), &'static str> {
    let child = usize::try_from(raw).map_err(|_| "expression edge references an unknown node")?;
    if child >= parent {
        return Err("expression edge is not canonical postorder");
    }
    let owner = owners.get_mut(child).ok_or("expression edge references an unknown node")?;
    *owner = owner.saturating_add(1);
    depths[parent] = depths[parent].max(depths[child].saturating_add(1));
    Ok(())
}
fn canonical_i32(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 || !value.is_ascii() {
        return false;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    (digits == "0" || (!digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())))
        && value != "-0"
}
fn canonical_string_literal(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first == '\'' || first == '"')
        && value.ends_with(first)
        && value.len() >= 2
        && !value[1..value.len() - 1]
            .chars()
            .any(|c| c == first || c == '\\' || c == '\r' || c == '\n')
}

fn verify_arena_order(
    raw: &RawFunctionBodySyntax,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    if raw.blocks.is_empty() {
        return;
    }
    let mut expected_block = 0usize;
    let mut expected_statement = 0usize;
    let mut seen = vec![false; raw.blocks.len()];
    let mut stack = vec![(0usize, 0usize, 1u32, false)];
    while let Some((block_id, offset, depth, entered)) = stack.pop() {
        if depth > MAX_NESTING_DEPTH {
            errors.limit("block nesting exceeds the protocol-v4 limit");
            break;
        }
        let Some(block) = raw.blocks.get(block_id) else {
            continue;
        };
        if !entered {
            if seen[block_id] {
                errors.node(path, "block graph has cyclic or shared reachability");
                continue;
            }
            seen[block_id] = true;
            if block_id != expected_block {
                errors.node(path, "block arena is not canonical preorder");
            }
            expected_block = expected_block.saturating_add(1);
        }
        let Some(statement_id) = block.statements.get(offset) else {
            continue;
        };
        stack.push((block_id, offset.saturating_add(1), depth, true));
        let Some(statement_index) = usize::try_from(*statement_id).ok() else {
            continue;
        };
        let Some(statement) = raw.statements.get(statement_index) else {
            continue;
        };
        if statement_index != expected_statement {
            errors.node(path, "statement arena is not canonical preorder");
        }
        expected_statement = expected_statement.saturating_add(1);
        match &statement.kind {
            RawStatementKind::Block { block } => push_block(*block, depth, &mut stack),
            RawStatementKind::If { then_block, else_clause, .. } => {
                if let Some(value) = else_clause {
                    push_block(value.block, depth, &mut stack);
                }
                push_block(*then_block, depth, &mut stack);
            }
            RawStatementKind::While { body_block, .. } => {
                push_block(*body_block, depth, &mut stack)
            }
            RawStatementKind::WeakUpgrade { success_block, failure_block, .. } => {
                push_block(*failure_block, depth, &mut stack);
                push_block(*success_block, depth, &mut stack);
            }
            _ => {}
        }
    }
    if seen.iter().any(|seen| !seen) {
        errors.node(path, "block arena has an unreachable block");
    }
}
fn push_block(raw: u32, parent_depth: u32, stack: &mut Vec<(usize, usize, u32, bool)>) {
    if let Ok(index) = usize::try_from(raw) {
        stack.push((index, 0, parent_depth.saturating_add(1), false));
    }
}

fn verify_provider_diagnostics(
    raw: Vec<RawProviderDiagnostic>,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Vec<Diagnostic> {
    let mut output = Vec::new();
    for value in raw {
        if value.code.is_empty()
            || value.code.chars().count() > 1024
            || value.message.is_empty()
            || value.message.chars().count() > 4096
            || value.guidance.is_empty()
            || value.guidance.chars().count() > 4096
        {
            errors.limit("provider diagnostic text exceeds its protocol-v4 limit");
            continue;
        }
        let span = match value.location {
            RawDiagnosticLocation::Global => None,
            RawDiagnosticLocation::Source { span } => match sources.verify_span(span) {
                Ok(span) => Some(span),
                Err(_) => {
                    errors.protocol(None, "provider diagnostic span is invalid");
                    continue;
                }
            },
        };
        output.push(match (value.severity, span) {
            (Severity::Error, Some(span)) => {
                Diagnostic::error_at(value.code, span, value.message, value.guidance)
            }
            (Severity::Warning, Some(span)) => {
                Diagnostic::warning_at(value.code, span, value.message, value.guidance)
            }
            (Severity::Error, None) => {
                Diagnostic::error(value.code, None, value.message, value.guidance)
            }
            (Severity::Warning, None) => {
                Diagnostic::warning(value.code, None, value.message, value.guidance)
            }
        });
    }
    output.sort_by_key(ToString::to_string);
    output
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use zryna_source::SourceFileInput;

    const SOURCE: &str = "function one(): i32 { return 1; }";
    fn uspan(source: &str, needle: &str) -> UntrustedSpan {
        let start = source.find(needle).unwrap();
        UntrustedSpan {
            file: 0,
            start: u32::try_from(start).unwrap(),
            end: u32::try_from(start + needle.len()).unwrap(),
        }
    }
    fn span_range(start: usize, end: usize) -> UntrustedSpan {
        UntrustedSpan {
            file: 0,
            start: u32::try_from(start).unwrap(),
            end: u32::try_from(end).unwrap(),
        }
    }
    fn sources(text: &str) -> SourceMap {
        SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text: text.into() }])
            .unwrap()
    }
    fn raw() -> RawProjectSyntaxSnapshot {
        let function = SOURCE.find("function").unwrap();
        let name = SOURCE.find("one").unwrap();
        let ty = SOURCE.find("i32").unwrap();
        let open = SOURCE.find('{').unwrap();
        let ret = SOURCE.find("return").unwrap();
        let literal = SOURCE.find('1').unwrap();
        let semi = SOURCE.find(';').unwrap();
        let close = SOURCE.rfind('}').unwrap();
        RawProjectSyntaxSnapshot {
            schema_version: 4,
            diagnostics: vec![],
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                type_syntax: vec![RawTypeSyntax {
                    span: span_range(ty, ty + 3),
                    kind: RawTypeSyntaxKind::Named {
                        name: RawIdentifierSyntax {
                            text: "i32".into(),
                            span: span_range(ty, ty + 3),
                        },
                    },
                }],
                data_declarations: vec![],
                functions: vec![RawFunctionSyntax {
                    span: span_range(0, SOURCE.len()),
                    export_span: None,
                    function_span: span_range(function, function + 8),
                    name: RawIdentifierSyntax {
                        text: "one".into(),
                        span: span_range(name, name + 3),
                    },
                    parameters: vec![],
                    result_type: 0,
                    body: RawFunctionBodySyntax {
                        span: span_range(open, close + 1),
                        root_block: 0,
                        blocks: vec![RawBlockSyntax {
                            span: span_range(open, close + 1),
                            open_brace_span: span_range(open, open + 1),
                            statements: vec![0],
                            close_brace_span: span_range(close, close + 1),
                        }],
                        statements: vec![RawStatementSyntax {
                            span: span_range(ret, semi + 1),
                            kind: RawStatementKind::Return {
                                keyword_span: span_range(ret, ret + 6),
                                value: 0,
                                semicolon_span: span_range(semi, semi + 1),
                            },
                        }],
                        expressions: vec![RawExpressionSyntax {
                            span: span_range(literal, literal + 1),
                            kind: RawExpressionKind::I32Literal { spelling: "1".into() },
                        }],
                    },
                }],
            }],
        }
    }

    fn nth(source: &str, needle: &str, occurrence: usize) -> usize {
        source.match_indices(needle).nth(occurrence).unwrap().0
    }
    fn named_type(source: &str, occurrence: usize) -> RawTypeSyntax {
        let start = nth(source, "i32", occurrence);
        RawTypeSyntax {
            span: span_range(start, start + 3),
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".into(),
                    span: span_range(start, start + 3),
                },
            },
        }
    }
    fn struct_decl(
        source: &str,
        name: &str,
        field: &str,
        occurrence: usize,
        type_id: u32,
    ) -> RawDataDeclaration {
        let interface = nth(source, "interface", occurrence);
        let name_start = source[interface..].find(name).unwrap() + interface;
        let extends = source[interface..].find("extends").unwrap() + interface;
        let marker = source[interface..].find("ZrynaStruct").unwrap() + interface;
        let open = source[interface..].find('{').unwrap() + interface;
        let close = source[open..].find('}').unwrap() + open;
        let field_start = source[open..].find(field).unwrap() + open;
        let colon = source[field_start..].find(':').unwrap() + field_start;
        let semi = source[colon..].find(';').unwrap() + colon;
        RawDataDeclaration {
            span: span_range(interface, close + 1),
            export_span: None,
            kind: RawDataDeclarationKind::Struct {
                interface_span: span_range(interface, interface + 9),
                name: RawIdentifierSyntax {
                    text: name.into(),
                    span: span_range(name_start, name_start + name.len()),
                },
                extends_span: span_range(extends, extends + 7),
                marker_span: span_range(marker, marker + 11),
                open_brace_span: span_range(open, open + 1),
                fields: vec![RawDataField {
                    span: span_range(field_start, semi + 1),
                    name: RawIdentifierSyntax {
                        text: field.into(),
                        span: span_range(field_start, field_start + field.len()),
                    },
                    colon_span: span_range(colon, colon + 1),
                    type_syntax: type_id,
                    semicolon_span: span_range(semi, semi + 1),
                }],
                close_brace_span: span_range(close, close + 1),
            },
        }
    }
    fn function_for(source: &str, type_id: u32) -> RawFunctionSyntax {
        let function = source.find("function").unwrap();
        let name = source[function..].find("one").unwrap() + function;
        let open = source[function..].find('{').unwrap() + function;
        let close = source[open..].find('}').unwrap() + open;
        let ret = source[open..].find("return").unwrap() + open;
        let literal = source[ret..].find('1').unwrap() + ret;
        let semi = source[ret..].find(';').unwrap() + ret;
        RawFunctionSyntax {
            span: span_range(function, close + 1),
            export_span: None,
            function_span: span_range(function, function + 8),
            name: RawIdentifierSyntax { text: "one".into(), span: span_range(name, name + 3) },
            parameters: vec![],
            result_type: type_id,
            body: RawFunctionBodySyntax {
                span: span_range(open, close + 1),
                root_block: 0,
                blocks: vec![RawBlockSyntax {
                    span: span_range(open, close + 1),
                    open_brace_span: span_range(open, open + 1),
                    statements: vec![0],
                    close_brace_span: span_range(close, close + 1),
                }],
                statements: vec![RawStatementSyntax {
                    span: span_range(ret, semi + 1),
                    kind: RawStatementKind::Return {
                        keyword_span: span_range(ret, ret + 6),
                        value: 0,
                        semicolon_span: span_range(semi, semi + 1),
                    },
                }],
                expressions: vec![RawExpressionSyntax {
                    span: span_range(literal, literal + 1),
                    kind: RawExpressionKind::I32Literal { spelling: "1".into() },
                }],
            },
        }
    }

    #[test]
    fn golden_decodes_verifies_and_remains_source_bound() {
        let raw = raw();
        let bytes = serde_json::to_vec(&raw).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        let authority = sources(SOURCE);
        let verified = verify_snapshot(decoded, &authority).unwrap();
        assert_eq!(verified.schema_version(), 4);
        assert!(verified.is_bound_to(&authority));
        assert!(!verified.is_bound_to(&sources(SOURCE)));
    }

    #[test]
    fn adapter_shorthand_fixture_decodes_and_verifies_end_to_end() {
        let source = include_str!("../../../tests/m3-fixtures/syntax-v4-shorthand.zry");
        let bytes = include_bytes!("../../../tests/m3-fixtures/syntax-v4-shorthand.json");
        let decoded = decode_snapshot(bytes).expect("adapter fixture must satisfy the closed DTO");
        let authority = sources(source);
        let verified = verify_snapshot(decoded, &authority)
            .expect("adapter shorthand value edge must satisfy Rust arena ownership");
        let fields = match &verified.files()[0].functions()[0].body.expressions[1].kind {
            RawExpressionKind::StructConstruction { fields, .. } => fields,
            other => panic!("expected struct construction, got {other:?}"),
        };
        assert!(matches!(fields[0].kind, RawFieldInitializerKind::Shorthand { value: 0, .. }));
    }

    #[test]
    fn decoder_is_exact_closed_and_bounded() {
        let mut value = serde_json::to_value(raw()).unwrap();
        value.as_object_mut().unwrap().insert("unknown".into(), true.into());
        assert_eq!(
            decode_snapshot(&serde_json::to_vec(&value).unwrap()),
            Err(SyntaxDecodeError::InvalidSnapshot)
        );
        assert_eq!(
            decode_snapshot(
                br#"{"schema_version":4,"schema_version":4,"files":[],"diagnostics":[]}"#
            ),
            Err(SyntaxDecodeError::InvalidSnapshot)
        );
        assert_eq!(
            reject_duplicate_json_keys(br#"{"outer":{"key":1,"key":2}}"#),
            Err(SyntaxDecodeError::InvalidSnapshot)
        );
        assert_eq!(
            decode_snapshot(&vec![b' '; MAX_RESPONSE_BYTES]),
            Err(SyntaxDecodeError::InvalidSnapshot)
        );
        assert!(matches!(
            decode_snapshot(&vec![b' '; MAX_RESPONSE_BYTES + 1]),
            Err(SyntaxDecodeError::ResponseTooLarge { .. })
        ));
    }

    #[test]
    fn version_and_source_claims_fail_closed() {
        let authority = sources(SOURCE);
        let mut wrong_version = raw();
        wrong_version.schema_version = 3;
        assert!(
            verify_snapshot(wrong_version, &authority)
                .unwrap_err()
                .iter()
                .any(|d| d.to_string().contains("ZRYNA-Y4001"))
        );
        let mut wrong_span = raw();
        wrong_span.files[0].functions[0].function_span = uspan(SOURCE, "one");
        assert!(
            verify_snapshot(wrong_span, &authority)
                .unwrap_err()
                .iter()
                .any(|d| d.to_string().contains("ZRYNA-Y4002"))
        );
    }

    #[test]
    fn source_structure_rejects_fabricated_call_and_type_claims() {
        let authority = sources(SOURCE);
        let mut call = raw();
        let function_name = call.files[0].functions[0].name.clone();
        let open = uspan(SOURCE, "(");
        let close = uspan(SOURCE, ")");
        call.files[0].functions[0].body.expressions[0].kind = RawExpressionKind::Call {
            callee: function_name,
            open_paren_span: open,
            arguments: vec![],
            close_paren_span: close,
        };
        assert!(verify_snapshot(call, &authority).is_err());

        let mut ty = raw();
        ty.files[0].type_syntax[0].kind =
            RawTypeSyntaxKind::Named { name: ty.files[0].functions[0].name.clone() };
        assert!(verify_snapshot(ty, &authority).is_err());
    }

    #[test]
    fn single_quoted_import_specifier_is_source_faithful() {
        const IMPORTED: &str =
            "import { dep } from './dep.zry';\nfunction one(): i32 { return 1; }";
        let import_end = IMPORTED.find(';').unwrap() + 1;
        let imported = IMPORTED.find("dep").unwrap();
        let from = IMPORTED.find("from").unwrap();
        let token_start = IMPORTED.find("'./dep.zry'").unwrap();
        let value_start = token_start + 1;
        let binding =
            RawIdentifierSyntax { text: "dep".into(), span: span_range(imported, imported + 3) };
        let snapshot = RawProjectSyntaxSnapshot {
            schema_version: 4,
            diagnostics: vec![],
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![RawImportSyntax {
                    span: span_range(0, import_end),
                    import_span: span_range(0, 6),
                    bindings: vec![RawImportBindingSyntax {
                        span: binding.span,
                        imported: binding.clone(),
                        local: binding,
                        as_span: None,
                    }],
                    from_span: span_range(from, from + 4),
                    specifier: RawModuleSpecifierSyntax {
                        text: "./dep.zry".into(),
                        token_span: span_range(token_start, token_start + 11),
                        value_span: span_range(value_start, value_start + 9),
                    },
                    semicolon_span: span_range(import_end - 1, import_end),
                }],
                type_syntax: vec![named_type(IMPORTED, 0)],
                data_declarations: vec![],
                functions: vec![function_for(IMPORTED, 0)],
            }],
        };
        assert!(verify_snapshot(snapshot, &sources(IMPORTED)).is_ok());
    }

    #[test]
    fn identifiers_use_the_portable_ascii_profile() {
        assert!(valid_identifier("_valid123"));
        assert!(!valid_identifier("9invalid"));
        assert!(!valid_identifier("with-dash"));
        assert!(!valid_identifier("café"));
        assert!(!valid_identifier(&"a".repeat(129)));
    }

    #[test]
    fn public_raw_vectors_enforce_every_exact_collection_limit() {
        let authority = sources(SOURCE);
        let accepted = |value: &RawProjectSyntaxSnapshot| check_budgets(value, &authority).is_ok();
        let zero = span_range(0, 0);
        let ident = RawIdentifierSyntax { text: "x".into(), span: zero };
        let binding = RawImportBindingSyntax {
            span: zero,
            imported: ident.clone(),
            local: ident.clone(),
            as_span: None,
        };
        let import = RawImportSyntax {
            span: zero,
            import_span: zero,
            bindings: vec![binding.clone()],
            from_span: zero,
            specifier: RawModuleSpecifierSyntax {
                text: "./x.zry".into(),
                token_span: zero,
                value_span: zero,
            },
            semicolon_span: zero,
        };
        let field = RawDataField {
            span: zero,
            name: ident.clone(),
            colon_span: zero,
            type_syntax: 0,
            semicolon_span: zero,
        };
        let variant = RawEnumVariant {
            span: zero,
            name: ident.clone(),
            colon_span: zero,
            payload_type: None,
            none_span: Some(zero),
            semicolon_span: zero,
        };
        let struct_declaration = RawDataDeclaration {
            span: zero,
            export_span: None,
            kind: RawDataDeclarationKind::Struct {
                interface_span: zero,
                name: ident.clone(),
                extends_span: zero,
                marker_span: zero,
                open_brace_span: zero,
                fields: vec![field.clone()],
                close_brace_span: zero,
            },
        };
        let enum_declaration = RawDataDeclaration {
            span: zero,
            export_span: None,
            kind: RawDataDeclarationKind::Enum {
                interface_span: zero,
                name: ident.clone(),
                extends_span: zero,
                marker_span: zero,
                open_brace_span: zero,
                variants: vec![variant.clone()],
                close_brace_span: zero,
            },
        };
        let diagnostic = RawProviderDiagnostic {
            code: "P1".into(),
            severity: Severity::Warning,
            location: RawDiagnosticLocation::Global,
            message: String::new(),
            guidance: String::new(),
        };

        let mut value = raw();
        value.files = vec![value.files[0].clone(); MAX_SOURCE_FILES];
        assert!(accepted(&value));
        value.files.push(value.files[0].clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.diagnostics = vec![diagnostic; MAX_PROVIDER_DIAGNOSTICS];
        assert!(accepted(&value));
        value.diagnostics.push(value.diagnostics[0].clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].imports = vec![import.clone(); MAX_IMPORTS_PER_MODULE];
        assert!(accepted(&value));
        value.files[0].imports.push(import.clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].imports = vec![import.clone()];
        value.files[0].imports[0].bindings =
            vec![binding.clone(); MAX_IMPORTED_NAMES_PER_DECLARATION];
        assert!(accepted(&value));
        value.files[0].imports[0].bindings.push(binding);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].type_syntax =
            vec![value.files[0].type_syntax[0].clone(); MAX_TYPE_NODES_PER_MODULE];
        assert!(accepted(&value));
        let extra = value.files[0].type_syntax[0].clone();
        value.files[0].type_syntax.push(extra);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].data_declarations =
            vec![struct_declaration.clone(); MAX_DATA_DECLARATIONS_PER_MODULE];
        assert!(accepted(&value));
        value.files[0].data_declarations.push(struct_declaration.clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].data_declarations = vec![struct_declaration.clone()];
        let RawDataDeclarationKind::Struct { fields, .. } =
            &mut value.files[0].data_declarations[0].kind
        else {
            unreachable!();
        };
        *fields = vec![field; MAX_MEMBERS_PER_DECLARATION];
        assert!(accepted(&value));
        let RawDataDeclarationKind::Struct { fields, .. } =
            &mut value.files[0].data_declarations[0].kind
        else {
            unreachable!();
        };
        fields.push(fields[0].clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].data_declarations = vec![enum_declaration];
        let RawDataDeclarationKind::Enum { variants, .. } =
            &mut value.files[0].data_declarations[0].kind
        else {
            unreachable!();
        };
        *variants = vec![variant; MAX_MEMBERS_PER_DECLARATION];
        assert!(accepted(&value));
        let RawDataDeclarationKind::Enum { variants, .. } =
            &mut value.files[0].data_declarations[0].kind
        else {
            unreachable!();
        };
        variants.push(variants[0].clone());
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].functions =
            vec![value.files[0].functions[0].clone(); MAX_FUNCTIONS_PER_MODULE];
        assert!(accepted(&value));
        let extra = value.files[0].functions[0].clone();
        value.files[0].functions.push(extra);
        assert!(!accepted(&value));

        let parameter = RawParameterSyntax { span: zero, name: ident.clone(), type_syntax: 0 };
        let mut value = raw();
        value.files[0].functions[0].parameters =
            vec![parameter.clone(); MAX_PARAMETERS_PER_FUNCTION];
        assert!(accepted(&value));
        value.files[0].functions[0].parameters.push(parameter);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].functions[0].body.blocks =
            vec![value.files[0].functions[0].body.blocks[0].clone(); MAX_BLOCKS_PER_FUNCTION];
        assert!(accepted(&value));
        let extra = value.files[0].functions[0].body.blocks[0].clone();
        value.files[0].functions[0].body.blocks.push(extra);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].functions[0].body.statements =
            vec![
                value.files[0].functions[0].body.statements[0].clone();
                MAX_STATEMENTS_PER_FUNCTION
            ];
        assert!(accepted(&value));
        let extra = value.files[0].functions[0].body.statements[0].clone();
        value.files[0].functions[0].body.statements.push(extra);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].functions[0].body.blocks[0].statements =
            vec![0; MAX_STATEMENTS_PER_FUNCTION];
        assert!(accepted(&value));
        value.files[0].functions[0].body.blocks[0].statements.push(0);
        assert!(!accepted(&value));

        let mut value = raw();
        value.files[0].functions[0].body.expressions =
            vec![
                value.files[0].functions[0].body.expressions[0].clone();
                MAX_EXPRESSIONS_PER_FUNCTION
            ];
        assert!(accepted(&value));
        let extra = value.files[0].functions[0].body.expressions[0].clone();
        value.files[0].functions[0].body.expressions.push(extra);
        assert!(!accepted(&value));

        let mut call = raw();
        call.files[0].functions[0].body.expressions[0].kind = RawExpressionKind::Call {
            callee: ident.clone(),
            open_paren_span: zero,
            arguments: vec![0; MAX_PARAMETERS_PER_FUNCTION],
            close_paren_span: zero,
        };
        assert!(accepted(&call));
        let RawExpressionKind::Call { arguments, .. } =
            &mut call.files[0].functions[0].body.expressions[0].kind
        else {
            unreachable!();
        };
        arguments.push(0);
        assert!(!accepted(&call));

        let initializer = RawFieldInitializer {
            span: zero,
            kind: RawFieldInitializerKind::Shorthand { name: ident.clone(), value: 0 },
        };
        let mut construction = raw();
        construction.files[0].functions[0].body.expressions[0].kind =
            RawExpressionKind::StructConstruction {
                type_name: ident.clone(),
                open_paren_span: zero,
                open_brace_span: zero,
                fields: vec![initializer.clone(); MAX_INITIALIZERS_PER_CONSTRUCTION],
                close_brace_span: zero,
                close_paren_span: zero,
            };
        assert!(accepted(&construction));
        let RawExpressionKind::StructConstruction { fields, .. } =
            &mut construction.files[0].functions[0].body.expressions[0].kind
        else {
            unreachable!();
        };
        fields.push(initializer);
        assert!(!accepted(&construction));

        for fixed in [false, true] {
            let mut construction = raw();
            let elements = vec![0; MAX_ELEMENTS_PER_CONSTRUCTION];
            construction.files[0].functions[0].body.expressions[0].kind = if fixed {
                RawExpressionKind::FixedArrayConstruction {
                    type_syntax: 0,
                    open_paren_span: zero,
                    open_bracket_span: zero,
                    elements,
                    close_bracket_span: zero,
                    close_paren_span: zero,
                }
            } else {
                RawExpressionKind::VecConstruction {
                    type_syntax: 0,
                    open_paren_span: zero,
                    open_bracket_span: zero,
                    elements,
                    close_bracket_span: zero,
                    close_paren_span: zero,
                }
            };
            assert!(accepted(&construction));
            match &mut construction.files[0].functions[0].body.expressions[0].kind {
                RawExpressionKind::FixedArrayConstruction { elements, .. }
                | RawExpressionKind::VecConstruction { elements, .. } => elements.push(0),
                _ => unreachable!(),
            }
            assert!(!accepted(&construction));
        }

        let arm = RawMatchArm {
            span: zero,
            type_name: ident.clone(),
            dot_span: zero,
            variant: ident,
            binding: None,
            arrow_span: zero,
            value: 0,
        };
        let mut matching = raw();
        matching.files[0].functions[0].body.expressions[0].kind = RawExpressionKind::Match {
            keyword_span: zero,
            open_paren_span: zero,
            scrutinee: 0,
            close_paren_span: zero,
            open_brace_span: zero,
            arms: vec![arm.clone(); MAX_MATCH_ARMS_PER_EXPRESSION],
            close_brace_span: zero,
        };
        assert!(accepted(&matching));
        let RawExpressionKind::Match { arms, .. } =
            &mut matching.files[0].functions[0].body.expressions[0].kind
        else {
            unreachable!();
        };
        arms.push(arm);
        assert!(!accepted(&matching));
    }

    #[test]
    fn top_level_arrays_allow_source_interleaving_but_reject_duplicate_names() {
        const INTERLEAVED: &str = "interface A extends ZrynaStruct { x: i32; }\nfunction one(): i32 { return 1; }\ninterface B extends ZrynaStruct { y: i32; }";
        let mut snapshot = RawProjectSyntaxSnapshot {
            schema_version: 4,
            diagnostics: vec![],
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                type_syntax: vec![
                    named_type(INTERLEAVED, 0),
                    named_type(INTERLEAVED, 1),
                    named_type(INTERLEAVED, 2),
                ],
                data_declarations: vec![
                    struct_decl(INTERLEAVED, "A", "x", 0, 0),
                    struct_decl(INTERLEAVED, "B", "y", 1, 2),
                ],
                functions: vec![function_for(INTERLEAVED, 1)],
            }],
        };
        let authority = sources(INTERLEAVED);
        assert!(verify_snapshot(snapshot.clone(), &authority).is_ok());
        if let RawDataDeclarationKind::Struct { name, .. } =
            &mut snapshot.files[0].data_declarations[1].kind
        {
            name.text = "A".into();
        }
        assert!(verify_snapshot(snapshot, &authority).is_err());
    }

    #[test]
    fn type_arena_rejects_orphans_forward_edges_sharing_and_depth() {
        let authority = sources(SOURCE);
        let mut orphan = raw();
        orphan.files[0]
            .type_syntax
            .push(RawTypeSyntax { span: span_range(0, 0), kind: RawTypeSyntaxKind::Missing });
        assert!(verify_snapshot(orphan, &authority).is_err());
        let mut owners = vec![0; 2];
        let mut depths = vec![1; 2];
        let path = NormalizedSourcePath::new("src/main.zry").unwrap();
        let mut errors = Errors::default();
        type_edge(1, 0, &mut owners, &mut depths, &path, &mut errors);
        assert!(!errors.items.is_empty());
        let mut owner = [0];
        own_type(0, &mut owner, &path, &mut errors);
        own_type(0, &mut owner, &path, &mut errors);
        assert_eq!(owner[0], 2);
        let mut chain_depth = vec![1; 130];
        let mut chain_owners = vec![0; 130];
        for parent in 1..130 {
            type_edge(
                u32::try_from(parent - 1).unwrap(),
                parent,
                &mut chain_owners,
                &mut chain_depth,
                &path,
                &mut errors,
            );
        }
        assert!(chain_depth[129] > MAX_NESTING_DEPTH);
    }

    #[test]
    fn graph_ownership_and_place_checks_are_non_recursive() {
        let reference = RawExpressionSyntax {
            span: span_range(0, 0),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "x".into(), span: span_range(0, 0) },
            },
        };
        let literal = RawExpressionSyntax {
            span: span_range(0, 0),
            kind: RawExpressionKind::I32Literal { spelling: "1".into() },
        };
        assert!(is_place(std::slice::from_ref(&reference), 0));
        assert!(!is_place(std::slice::from_ref(&literal), 0));
        let mut owners = [0];
        let mut depths = [1];
        assert!(expression_edge(0, 0, &mut owners, &mut depths).is_err());
        let mut roots = [0];
        assert!(own_expression_root(0, 1, &mut roots));
        assert!(own_expression_root(0, 1, &mut roots));
        assert_eq!(roots[0], 2);
    }

    #[test]
    fn fixed_array_length_has_exact_profile_boundary() {
        assert!(canonical_u32("0"));
        assert!(canonical_u32("1048576"));
        assert!(!canonical_u32("1048577"));
        assert!(!canonical_u32("01"));
        assert!(!canonical_u32("4294967295"));
    }

    #[test]
    fn integer_spelling_is_syntax_only() {
        assert!(canonical_i32("2147483648"));
        assert!(canonical_i32("-2147483649"));
        assert!(!canonical_i32("01"));
        assert!(!canonical_i32("-0"));
    }

    #[test]
    fn sensitive_names_and_diagnostics_are_deterministic_and_bounded() {
        assert!(is_sensitive("__proto__"));
        assert!(is_sensitive("prototype"));
        assert!(is_sensitive("constructor"));
        assert!(!is_sensitive("ordinary"));
        let authority = sources(SOURCE);
        let mut value = raw();
        value.diagnostics = (0..MAX_PROVIDER_DIAGNOSTICS)
            .rev()
            .map(|index| RawProviderDiagnostic {
                code: format!("P{index:03}"),
                severity: Severity::Warning,
                location: RawDiagnosticLocation::Global,
                message: "message".into(),
                guidance: "guidance".into(),
            })
            .collect();
        let verified = verify_snapshot(value, &authority).unwrap();
        let rendered = verified.diagnostics().iter().map(ToString::to_string).collect::<Vec<_>>();
        let mut sorted = rendered.clone();
        sorted.sort();
        assert_eq!(rendered, sorted);
    }
}
