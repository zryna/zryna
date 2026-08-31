//! Fail-closed provider-neutral syntax protocol version 3.

use std::{fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, IgnoredAny, SeqAccess, Visitor},
};
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_source::{
    FileId, MAX_SOURCE_FILES, NormalizedSourcePath, SourceMap, SourceMapIdentity, Span,
    UntrustedSpan,
};

/// Exact syntax wire protocol version.
pub const PROTOCOL_VERSION: u32 = 3;
/// Maximum encoded response size.
pub const MAX_RESPONSE_BYTES: usize = 64 * 1_024 * 1_024;
/// Maximum aggregate authoritative source size.
pub const MAX_AGGREGATE_SOURCE_BYTES: usize = 8 * 1_024 * 1_024;
/// Maximum imports in one module.
pub const MAX_IMPORTS_PER_MODULE: usize = 4_096;
/// Maximum imports in one project.
pub const MAX_IMPORTS_PER_PROJECT: usize = 65_536;
/// Maximum bindings in one import declaration.
pub const MAX_IMPORTED_NAMES_PER_DECLARATION: usize = 256;
/// Maximum imported names in one project.
pub const MAX_IMPORTED_NAMES_PER_PROJECT: usize = 65_536;
/// Maximum functions in one module.
pub const MAX_FUNCTIONS_PER_MODULE: usize = 4_096;
/// Maximum functions in one project.
pub const MAX_FUNCTIONS_PER_PROJECT: usize = 16_384;
/// Maximum parameters in one function.
pub const MAX_PARAMETERS_PER_FUNCTION: usize = 256;
/// Maximum parameters in one project.
pub const MAX_PARAMETERS_PER_PROJECT: usize = 262_144;
/// Maximum blocks in one function.
pub const MAX_BLOCKS_PER_FUNCTION: usize = 4_096;
/// Maximum blocks in one project.
pub const MAX_BLOCKS_PER_PROJECT: usize = 65_536;
/// Maximum statements in one function or block edge list.
pub const MAX_STATEMENTS_PER_FUNCTION: usize = 4_096;
/// Maximum statements in one project.
pub const MAX_STATEMENTS_PER_PROJECT: usize = 65_536;
/// Maximum expressions in one function.
pub const MAX_EXPRESSIONS_PER_FUNCTION: usize = 16_384;
/// Maximum expressions in one project.
pub const MAX_EXPRESSIONS_PER_PROJECT: usize = 262_144;
/// Maximum local declarations in one function.
pub const MAX_LOCALS_PER_FUNCTION: usize = 4_096;
/// Maximum local declarations in one project.
pub const MAX_LOCALS_PER_PROJECT: usize = 65_536;
/// Maximum verified syntax nesting depth.
pub const MAX_NESTING_DEPTH: u32 = 128;
/// Maximum UTF-8 byte length of a module specifier.
pub const MAX_MODULE_SPECIFIER_BYTES: usize = 1_024;
/// Maximum Unicode scalar count for a name.
pub const MAX_NAME_CHARACTERS: usize = 1_024;
/// Maximum byte length of an integer literal spelling.
pub const MAX_LITERAL_BYTES: usize = 64;
/// Maximum provider diagnostics in a response.
pub const MAX_PROVIDER_DIAGNOSTICS: usize = 256;
/// Maximum Unicode scalar count in diagnostic text.
pub const MAX_DIAGNOSTIC_TEXT_CHARACTERS: usize = 4_096;
/// Maximum deterministic verifier diagnostics.
pub const MAX_VALIDATION_ERRORS: usize = 256;

/// Untrusted top-level protocol-v3 response DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProjectSyntaxSnapshot {
    /// Exact schema version.
    pub schema_version: u32,
    /// Dense source units.
    #[serde(deserialize_with = "deserialize_files")]
    pub files: Vec<RawSourceUnit>,
    #[serde(deserialize_with = "deserialize_diagnostics")]
    /// Provider diagnostics.
    pub diagnostics: Vec<RawProviderDiagnostic>,
}

/// Untrusted source-unit DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawSourceUnit {
    /// Dense authoritative file id.
    pub id: u32,
    /// Portable normalized path.
    pub path: String,
    /// Imports in source order.
    #[serde(deserialize_with = "deserialize_imports")]
    pub imports: Vec<RawImportSyntax>,
    #[serde(deserialize_with = "deserialize_functions")]
    /// Functions in source order.
    pub functions: Vec<RawFunctionSyntax>,
}

/// Untrusted import DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawImportSyntax {
    /// Complete declaration span.
    pub span: UntrustedSpan,
    /// `import` token span.
    pub import_span: UntrustedSpan,
    /// Imported bindings.
    #[serde(deserialize_with = "deserialize_imported_names")]
    pub bindings: Vec<RawImportBindingSyntax>,
    /// `from` token span.
    pub from_span: UntrustedSpan,
    /// Quoted module specifier.
    pub specifier: RawModuleSpecifierSyntax,
    /// Semicolon token span.
    pub semicolon_span: UntrustedSpan,
}

/// Untrusted named-import binding DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawImportBindingSyntax {
    /// Complete binding span.
    pub span: UntrustedSpan,
    /// Imported name.
    pub imported: RawIdentifierSyntax,
    /// Local name.
    pub local: RawIdentifierSyntax,
    /// Optional `as` token span.
    pub as_span: Option<UntrustedSpan>,
}

/// Untrusted module-specifier DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawModuleSpecifierSyntax {
    /// Unquoted specifier text.
    pub text: String,
    /// Quoted token span.
    pub token_span: UntrustedSpan,
    /// Unquoted value span.
    pub value_span: UntrustedSpan,
}

/// Untrusted function DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionSyntax {
    /// Complete function span.
    pub span: UntrustedSpan,
    /// Optional `export` token span.
    pub export_span: Option<UntrustedSpan>,
    /// `function` token span.
    pub function_span: UntrustedSpan,
    /// Declared name.
    pub name: RawIdentifierSyntax,
    /// Ordered parameters.
    #[serde(deserialize_with = "deserialize_parameters")]
    pub parameters: Vec<RawParameterSyntax>,
    /// Result type syntax.
    pub result_type: RawTypeSyntax,
    /// Function body arenas.
    pub body: RawFunctionBodySyntax,
}

/// Untrusted identifier DTO.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawIdentifierSyntax {
    /// Identifier spelling.
    pub text: String,
    /// Identifier token span.
    pub span: UntrustedSpan,
}

/// Untrusted parameter DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawParameterSyntax {
    /// Complete parameter span.
    pub span: UntrustedSpan,
    /// Parameter name.
    pub name: RawIdentifierSyntax,
    /// Parameter type.
    pub type_syntax: RawTypeSyntax,
}

/// Untrusted type-syntax DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawTypeSyntax {
    /// Complete type span.
    pub span: UntrustedSpan,
    /// Type form.
    pub kind: RawTypeSyntaxKind,
}

/// Untrusted type form.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawTypeSyntaxKind {
    /// A provider-observed missing type annotation.
    Missing,
    /// A source-spelled named type.
    Named {
        /// Exact source spelling of the type name.
        name: String,
    },
}

/// Untrusted function-body arena DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawFunctionBodySyntax {
    /// Complete body span.
    pub span: UntrustedSpan,
    /// Root block id, required to be zero.
    pub root_block: u32,
    /// Canonical block arena.
    #[serde(deserialize_with = "deserialize_blocks")]
    pub blocks: Vec<RawBlockSyntax>,
    #[serde(deserialize_with = "deserialize_statements")]
    /// Canonical statement arena.
    pub statements: Vec<RawStatementSyntax>,
    #[serde(deserialize_with = "deserialize_expressions")]
    /// Canonical expression arena.
    pub expressions: Vec<RawExpressionSyntax>,
}

/// Untrusted block DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawBlockSyntax {
    /// Complete block span.
    pub span: UntrustedSpan,
    /// Opening brace span.
    pub open_brace_span: UntrustedSpan,
    /// Owned statement ids.
    #[serde(deserialize_with = "deserialize_statement_ids")]
    pub statements: Vec<u32>,
    /// Closing brace span.
    pub close_brace_span: UntrustedSpan,
}

/// Untrusted statement DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawStatementSyntax {
    /// Complete statement span.
    pub span: UntrustedSpan,
    /// Statement form.
    pub kind: RawStatementKind,
}

/// Untrusted statement form.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawStatementKind {
    /// A `let` or `const` declaration with an explicit type and initializer.
    LocalDeclaration {
        /// Declaration-keyword token span.
        keyword_span: UntrustedSpan,
        /// Whether the declaration is mutable.
        mutable: bool,
        /// Declared local name.
        name: RawIdentifierSyntax,
        /// Declared local type syntax.
        type_syntax: RawTypeSyntax,
        /// Equals-token span.
        equals_span: UntrustedSpan,
        /// Initializer expression arena index.
        initializer: u32,
        /// Semicolon-token span.
        semicolon_span: UntrustedSpan,
    },
    /// An assignment to a local binding.
    Assignment {
        /// Assignment target.
        target: RawIdentifierSyntax,
        /// Equals-token span.
        equals_span: UntrustedSpan,
        /// Assigned expression arena index.
        value: u32,
        /// Semicolon-token span.
        semicolon_span: UntrustedSpan,
    },
    /// A value-returning statement.
    Return {
        /// `return` token span.
        keyword_span: UntrustedSpan,
        /// Returned expression arena index.
        value: u32,
        /// Semicolon-token span.
        semicolon_span: UntrustedSpan,
    },
    /// A nested block statement.
    Block {
        /// Owned block arena index.
        block: u32,
    },
    /// A conditional statement with an optional `else` clause.
    If {
        /// `if` token span.
        keyword_span: UntrustedSpan,
        /// Opening-parenthesis token span.
        open_paren_span: UntrustedSpan,
        /// Condition expression arena index.
        condition: u32,
        /// Closing-parenthesis token span.
        close_paren_span: UntrustedSpan,
        /// Then-block arena index.
        then_block: u32,
        /// Optional else clause.
        else_clause: Option<RawElseSyntax>,
    },
    /// A `while` loop.
    While {
        /// `while` token span.
        keyword_span: UntrustedSpan,
        /// Opening-parenthesis token span.
        open_paren_span: UntrustedSpan,
        /// Condition expression arena index.
        condition: u32,
        /// Closing-parenthesis token span.
        close_paren_span: UntrustedSpan,
        /// Loop-body block arena index.
        body_block: u32,
    },
}

/// Untrusted else-clause DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawElseSyntax {
    /// `else` token span.
    pub keyword_span: UntrustedSpan,
    /// Owned block id.
    pub block: u32,
}

/// Untrusted expression DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawExpressionSyntax {
    /// Complete expression span.
    pub span: UntrustedSpan,
    /// Expression form.
    pub kind: RawExpressionKind,
}

/// Untrusted expression form.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawExpressionKind {
    /// A name reference.
    Reference {
        /// Referenced identifier.
        name: RawIdentifierSyntax,
    },
    /// A Boolean literal.
    BoolLiteral {
        /// Literal value.
        value: bool,
    },
    /// A signed 32-bit integer literal spelling.
    I32Literal {
        /// Exact source spelling.
        spelling: String,
    },
    /// Prefix arithmetic negation.
    Negation {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Operand expression arena index.
        operand: u32,
    },
    /// Wrapping 32-bit integer addition.
    Addition {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Wrapping 32-bit integer subtraction.
    Subtraction {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Wrapping 32-bit integer multiplication.
    Multiplication {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Scalar equality comparison.
    Equal {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Scalar inequality comparison.
    NotEqual {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Signed less-than comparison.
    LessThan {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Signed less-than-or-equal comparison.
    LessEqual {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Signed greater-than comparison.
    GreaterThan {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// Signed greater-than-or-equal comparison.
    GreaterEqual {
        /// Operator-token span.
        operator_span: UntrustedSpan,
        /// Left operand expression arena index.
        lhs: u32,
        /// Right operand expression arena index.
        rhs: u32,
    },
    /// A direct function call.
    Call {
        /// Called function name.
        callee: RawIdentifierSyntax,
        /// Opening-parenthesis token span.
        open_paren_span: UntrustedSpan,
        /// Argument expression arena indices.
        #[serde(deserialize_with = "deserialize_call_arguments")]
        arguments: Vec<u32>,
        /// Closing-parenthesis token span.
        close_paren_span: UntrustedSpan,
    },
}

/// Untrusted provider diagnostic DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawProviderDiagnostic {
    /// Stable provider diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: Severity,
    /// Diagnostic location.
    pub location: RawDiagnosticLocation,
    /// Human-readable message.
    pub message: String,
    /// Remediation guidance.
    pub guidance: String,
}

/// Untrusted diagnostic location.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawDiagnosticLocation {
    /// A diagnostic not associated with source text.
    Global,
    /// A diagnostic associated with an untrusted source span.
    Source {
        /// Claimed primary source span.
        span: UntrustedSpan,
    },
}

/// Opaque protocol-v3 project syntax verified against one exact source map.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectSyntaxSnapshot {
    #[serde(skip)]
    source_map_identity: SourceMapIdentity,
    schema_version: u32,
    files: Vec<SourceUnit>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectSyntaxSnapshot {
    /// Returns the verified schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    /// Returns verified source units in dense file-id order.
    #[must_use]
    pub fn files(&self) -> &[SourceUnit] {
        &self.files
    }
    /// Returns verified provider diagnostics in deterministic order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Returns whether this snapshot remains bound to the supplied exact source map.
    #[must_use]
    pub fn is_bound_to(&self, sources: &SourceMap) -> bool {
        self.source_map_identity == sources.identity()
            && self.files.len() == sources.len()
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

/// One verified protocol-v3 source unit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceUnit {
    id: FileId,
    path: NormalizedSourcePath,
    imports: Vec<ImportSyntax>,
    functions: Vec<FunctionSyntax>,
}

impl SourceUnit {
    /// Returns the authoritative source-map file id.
    #[must_use]
    pub const fn id(&self) -> FileId {
        self.id
    }
    /// Returns the authoritative normalized source path.
    #[must_use]
    pub const fn path(&self) -> &NormalizedSourcePath {
        &self.path
    }
    /// Returns verified imports in source order.
    #[must_use]
    pub fn imports(&self) -> &[ImportSyntax] {
        &self.imports
    }
    /// Returns verified functions in source order.
    #[must_use]
    pub fn functions(&self) -> &[FunctionSyntax] {
        &self.functions
    }
}

/// One verified named-import declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImportSyntax {
    span: Span,
    import_span: Span,
    bindings: Vec<ImportBindingSyntax>,
    from_span: Span,
    specifier: ModuleSpecifierSyntax,
    semicolon_span: Span,
}

impl ImportSyntax {
    /// Returns the complete declaration span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the verified `import` token span.
    #[must_use]
    pub const fn import_span(&self) -> Span {
        self.import_span
    }
    /// Returns imported bindings in source order.
    #[must_use]
    pub fn bindings(&self) -> &[ImportBindingSyntax] {
        &self.bindings
    }
    /// Returns the verified module specifier.
    #[must_use]
    pub const fn specifier(&self) -> &ModuleSpecifierSyntax {
        &self.specifier
    }
    /// Returns the verified `from` token span.
    #[must_use]
    pub const fn from_span(&self) -> Span {
        self.from_span
    }
    /// Returns the verified semicolon token span.
    #[must_use]
    pub const fn semicolon_span(&self) -> Span {
        self.semicolon_span
    }
}

/// One verified named-import binding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImportBindingSyntax {
    span: Span,
    imported: IdentifierSyntax,
    local: IdentifierSyntax,
    as_span: Option<Span>,
}

impl ImportBindingSyntax {
    /// Returns the complete verified binding span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the imported source name.
    #[must_use]
    pub const fn imported(&self) -> &IdentifierSyntax {
        &self.imported
    }
    /// Returns the local binding name.
    #[must_use]
    pub const fn local(&self) -> &IdentifierSyntax {
        &self.local
    }
    /// Returns the optional `as` token span.
    #[must_use]
    pub const fn as_span(&self) -> Option<Span> {
        self.as_span
    }
}

/// One verified relative `.zry` module specifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModuleSpecifierSyntax {
    text: String,
    token_span: Span,
    value_span: Span,
}

impl ModuleSpecifierSyntax {
    /// Returns the unquoted specifier text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Returns the complete quoted token span.
    #[must_use]
    pub const fn token_span(&self) -> Span {
        self.token_span
    }
    /// Returns the unquoted value span.
    #[must_use]
    pub const fn value_span(&self) -> Span {
        self.value_span
    }
}

/// One verified function declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FunctionSyntax {
    span: Span,
    export_span: Option<Span>,
    function_span: Span,
    name: IdentifierSyntax,
    parameters: Vec<ParameterSyntax>,
    result_type: TypeSyntax,
    body: FunctionBodySyntax,
}

impl FunctionSyntax {
    /// Returns the complete function declaration span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the optional `export` token span.
    #[must_use]
    pub const fn export_span(&self) -> Option<Span> {
        self.export_span
    }
    /// Returns the verified `function` token span.
    #[must_use]
    pub const fn function_span(&self) -> Span {
        self.function_span
    }
    /// Returns the declared function name.
    #[must_use]
    pub const fn name(&self) -> &IdentifierSyntax {
        &self.name
    }
    /// Returns parameters in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterSyntax] {
        &self.parameters
    }
    /// Returns the declared result type syntax.
    #[must_use]
    pub const fn result_type(&self) -> &TypeSyntax {
        &self.result_type
    }
    /// Returns the verified function-body arenas.
    #[must_use]
    pub const fn body(&self) -> &FunctionBodySyntax {
        &self.body
    }
}

/// A verified source-spelled identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdentifierSyntax {
    text: String,
    span: Span,
}
impl IdentifierSyntax {
    /// Returns the identifier spelling.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Returns the identifier token span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// One verified function parameter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParameterSyntax {
    span: Span,
    name: IdentifierSyntax,
    type_syntax: TypeSyntax,
}
impl ParameterSyntax {
    /// Returns the complete parameter span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the parameter name.
    #[must_use]
    pub const fn name(&self) -> &IdentifierSyntax {
        &self.name
    }
    /// Returns the declared parameter type syntax.
    #[must_use]
    pub const fn type_syntax(&self) -> &TypeSyntax {
        &self.type_syntax
    }
}

/// One verified type-syntax node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TypeSyntax {
    span: Span,
    kind: TypeSyntaxKind,
}
impl TypeSyntax {
    /// Returns the complete type span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the verified type form.
    #[must_use]
    pub const fn kind(&self) -> &TypeSyntaxKind {
        &self.kind
    }
}

/// A verified source-level type form.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TypeSyntaxKind {
    /// A provider-observed missing type annotation.
    Missing,
    /// A source-spelled named type.
    Named {
        /// Exact source spelling of the type name.
        name: String,
    },
}

/// Dense verified function-body block identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct BlockId(u32);
impl BlockId {
    /// Returns the zero-based arena index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}
/// Dense verified function-body statement identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct StatementId(u32);
impl StatementId {
    /// Returns the zero-based arena index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}
/// Dense verified function-body expression identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ExpressionId(u32);
impl ExpressionId {
    /// Returns the zero-based arena index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Verified canonical block, statement, and expression arenas for one function.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FunctionBodySyntax {
    span: Span,
    root_block: BlockId,
    blocks: Vec<BlockSyntax>,
    statements: Vec<StatementSyntax>,
    expressions: Vec<ExpressionSyntax>,
}
impl FunctionBodySyntax {
    /// Returns the complete body span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the root block identifier, which is always index zero.
    #[must_use]
    pub const fn root_block(&self) -> BlockId {
        self.root_block
    }
    /// Returns the canonical block arena.
    #[must_use]
    pub fn blocks(&self) -> &[BlockSyntax] {
        &self.blocks
    }
    /// Returns the canonical statement arena.
    #[must_use]
    pub fn statements(&self) -> &[StatementSyntax] {
        &self.statements
    }
    /// Returns the canonical expression arena.
    #[must_use]
    pub fn expressions(&self) -> &[ExpressionSyntax] {
        &self.expressions
    }
}

/// One verified block in a function-body arena.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BlockSyntax {
    span: Span,
    open_brace_span: Span,
    statements: Vec<StatementId>,
    close_brace_span: Span,
}
impl BlockSyntax {
    /// Returns the complete block span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the verified opening-brace token span.
    #[must_use]
    pub const fn open_brace_span(&self) -> Span {
        self.open_brace_span
    }
    /// Returns statement identifiers owned by this block in source order.
    #[must_use]
    pub fn statements(&self) -> &[StatementId] {
        &self.statements
    }
    /// Returns the verified closing-brace token span.
    #[must_use]
    pub const fn close_brace_span(&self) -> Span {
        self.close_brace_span
    }
}

/// One verified statement in a function-body arena.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatementSyntax {
    span: Span,
    kind: StatementKind,
}
impl StatementSyntax {
    /// Returns the complete statement span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the verified statement form.
    #[must_use]
    pub const fn kind(&self) -> &StatementKind {
        &self.kind
    }
}

/// A verified source-level statement form.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StatementKind {
    /// A `let` or `const` declaration with an explicit type and initializer.
    LocalDeclaration {
        /// Declaration-keyword token span.
        keyword_span: Span,
        /// Whether the declaration is mutable.
        mutable: bool,
        /// Declared local name.
        name: IdentifierSyntax,
        /// Declared local type syntax.
        type_syntax: TypeSyntax,
        /// Equals-token span.
        equals_span: Span,
        /// Initializer expression identifier.
        initializer: ExpressionId,
        /// Semicolon-token span.
        semicolon_span: Span,
    },
    /// An assignment to a local binding.
    Assignment {
        /// Assignment target.
        target: IdentifierSyntax,
        /// Equals-token span.
        equals_span: Span,
        /// Assigned expression identifier.
        value: ExpressionId,
        /// Semicolon-token span.
        semicolon_span: Span,
    },
    /// A value-returning statement.
    Return {
        /// `return` token span.
        keyword_span: Span,
        /// Returned expression identifier.
        value: ExpressionId,
        /// Semicolon-token span.
        semicolon_span: Span,
    },
    /// A nested block statement.
    Block {
        /// Owned block identifier.
        block: BlockId,
    },
    /// A conditional statement with an optional `else` clause.
    If {
        /// `if` token span.
        keyword_span: Span,
        /// Opening-parenthesis token span.
        open_paren_span: Span,
        /// Condition expression identifier.
        condition: ExpressionId,
        /// Closing-parenthesis token span.
        close_paren_span: Span,
        /// Then-block identifier.
        then_block: BlockId,
        /// Optional else clause.
        else_clause: Option<ElseSyntax>,
    },
    /// A `while` loop.
    While {
        /// `while` token span.
        keyword_span: Span,
        /// Opening-parenthesis token span.
        open_paren_span: Span,
        /// Condition expression identifier.
        condition: ExpressionId,
        /// Closing-parenthesis token span.
        close_paren_span: Span,
        /// Loop-body block identifier.
        body_block: BlockId,
    },
}

/// One verified `else` clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ElseSyntax {
    keyword_span: Span,
    block: BlockId,
}

impl ElseSyntax {
    /// Returns the verified `else` token span.
    #[must_use]
    pub const fn keyword_span(&self) -> Span {
        self.keyword_span
    }

    /// Returns the verified block owned by this clause.
    #[must_use]
    pub const fn block(&self) -> BlockId {
        self.block
    }
}

/// One verified expression in a function-body arena.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExpressionSyntax {
    span: Span,
    kind: ExpressionKind,
}
impl ExpressionSyntax {
    /// Returns the complete expression span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Returns the verified expression form.
    #[must_use]
    pub const fn kind(&self) -> &ExpressionKind {
        &self.kind
    }
}

/// A verified source-level expression form.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ExpressionKind {
    /// A name reference.
    Reference {
        /// Referenced identifier.
        name: IdentifierSyntax,
    },
    /// A Boolean literal.
    BoolLiteral {
        /// Literal value.
        value: bool,
    },
    /// A signed 32-bit integer literal spelling.
    I32Literal {
        /// Exact source spelling.
        spelling: String,
    },
    /// Prefix arithmetic negation.
    Negation {
        /// Operator-token span.
        operator_span: Span,
        /// Operand expression identifier.
        operand: ExpressionId,
    },
    /// Wrapping 32-bit integer addition.
    Addition {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Wrapping 32-bit integer subtraction.
    Subtraction {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Wrapping 32-bit integer multiplication.
    Multiplication {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Scalar equality comparison.
    Equal {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Scalar inequality comparison.
    NotEqual {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Signed less-than comparison.
    LessThan {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Signed less-than-or-equal comparison.
    LessEqual {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Signed greater-than comparison.
    GreaterThan {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// Signed greater-than-or-equal comparison.
    GreaterEqual {
        /// Operator-token span.
        operator_span: Span,
        /// Left operand expression identifier.
        lhs: ExpressionId,
        /// Right operand expression identifier.
        rhs: ExpressionId,
    },
    /// A direct function call.
    Call {
        /// Called function name.
        callee: IdentifierSyntax,
        /// Opening-parenthesis token span.
        open_paren_span: Span,
        /// Argument expression identifiers in source order.
        arguments: Vec<ExpressionId>,
        /// Closing-parenthesis token span.
        close_paren_span: Span,
    },
}

/// Failure while decoding one untrusted protocol-v3 response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxDecodeError {
    /// The encoded response exceeds the fixed byte budget.
    ResponseTooLarge {
        /// Observed response size in bytes.
        actual: usize,
        /// Maximum accepted response size in bytes.
        limit: usize,
    },
    /// The response is not exact protocol-v3 JSON.
    InvalidSnapshot,
}
impl SyntaxDecodeError {
    /// Returns the stable diagnostic code for this decoding failure.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ResponseTooLarge { .. } => "ZRYNA-F1201",
            Self::InvalidSnapshot => "ZRYNA-F1103",
        }
    }
}
impl fmt::Display for SyntaxDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseTooLarge { actual, limit } => {
                write!(formatter, "syntax response contains {actual} bytes; the limit is {limit}")
            }
            Self::InvalidSnapshot => {
                formatter.write_str("syntax response is not exact protocol-v3 JSON")
            }
        }
    }
}
impl std::error::Error for SyntaxDecodeError {}

fn deserialize_files<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<RawSourceUnit>, D::Error> {
    deserialize_bounded::<D, RawSourceUnit, MAX_SOURCE_FILES>(d, "source files")
}
fn deserialize_imports<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<RawImportSyntax>, D::Error> {
    deserialize_bounded::<D, RawImportSyntax, MAX_IMPORTS_PER_MODULE>(d, "imports")
}
fn deserialize_imported_names<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawImportBindingSyntax>, D::Error> {
    deserialize_bounded::<D, RawImportBindingSyntax, MAX_IMPORTED_NAMES_PER_DECLARATION>(
        d,
        "imported names",
    )
}
fn deserialize_functions<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawFunctionSyntax>, D::Error> {
    deserialize_bounded::<D, RawFunctionSyntax, MAX_FUNCTIONS_PER_MODULE>(d, "functions")
}
fn deserialize_parameters<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawParameterSyntax>, D::Error> {
    deserialize_bounded::<D, RawParameterSyntax, MAX_PARAMETERS_PER_FUNCTION>(d, "parameters")
}
fn deserialize_blocks<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<RawBlockSyntax>, D::Error> {
    deserialize_bounded::<D, RawBlockSyntax, MAX_BLOCKS_PER_FUNCTION>(d, "blocks")
}
fn deserialize_statements<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawStatementSyntax>, D::Error> {
    deserialize_bounded::<D, RawStatementSyntax, MAX_STATEMENTS_PER_FUNCTION>(d, "statements")
}
fn deserialize_statement_ids<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
    deserialize_bounded::<D, u32, MAX_STATEMENTS_PER_FUNCTION>(d, "block statements")
}
fn deserialize_expressions<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawExpressionSyntax>, D::Error> {
    deserialize_bounded::<D, RawExpressionSyntax, MAX_EXPRESSIONS_PER_FUNCTION>(d, "expressions")
}
fn deserialize_call_arguments<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
    deserialize_bounded::<D, u32, MAX_PARAMETERS_PER_FUNCTION>(d, "call arguments")
}
fn deserialize_diagnostics<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<RawProviderDiagnostic>, D::Error> {
    deserialize_bounded::<D, RawProviderDiagnostic, MAX_PROVIDER_DIAGNOSTICS>(d, "diagnostics")
}

fn deserialize_bounded<'de, D, T, const MAX: usize>(
    d: D,
    label: &'static str,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const MAX: usize> {
        label: &'static str,
        marker: PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Bounded<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {MAX} {}", self.label)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(MAX));
            while values.len() < MAX {
                match seq.next_element()? {
                    Some(value) => values.push(value),
                    None => return Ok(values),
                }
            }
            if seq.next_element::<IgnoredAny>()?.is_some() {
                return Err(A::Error::custom(format_args!("{} exceeds limit {MAX}", self.label)));
            }
            Ok(values)
        }
    }
    d.deserialize_seq(Bounded::<T, MAX> { label, marker: PhantomData })
}

/// Decodes an exact, bounded protocol-v3 response without trusting its semantic claims.
///
/// # Errors
///
/// Returns [`SyntaxDecodeError::ResponseTooLarge`] when `bytes` exceeds the response budget, or
/// [`SyntaxDecodeError::InvalidSnapshot`] when it is not exact protocol-v3 JSON.
pub fn decode_snapshot(bytes: &[u8]) -> Result<RawProjectSyntaxSnapshot, SyntaxDecodeError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(SyntaxDecodeError::ResponseTooLarge {
            actual: bytes.len(),
            limit: MAX_RESPONSE_BYTES,
        });
    }
    serde_json::from_slice(bytes).map_err(|_| SyntaxDecodeError::InvalidSnapshot)
}

#[derive(Default)]
struct Counts {
    imports: usize,
    imported_names: usize,
    functions: usize,
    parameters: usize,
    blocks: usize,
    statements: usize,
    expressions: usize,
    locals: usize,
}

fn exceeds_project_limits(counts: &Counts) -> bool {
    counts.imports > MAX_IMPORTS_PER_PROJECT
        || counts.imported_names > MAX_IMPORTED_NAMES_PER_PROJECT
        || counts.functions > MAX_FUNCTIONS_PER_PROJECT
        || counts.parameters > MAX_PARAMETERS_PER_PROJECT
        || counts.blocks > MAX_BLOCKS_PER_PROJECT
        || counts.statements > MAX_STATEMENTS_PER_PROJECT
        || counts.expressions > MAX_EXPRESSIONS_PER_PROJECT
        || counts.locals > MAX_LOCALS_PER_PROJECT
}

fn checked_add(total: &mut usize, value: usize) -> bool {
    if let Some(next) = total.checked_add(value) {
        *total = next;
        true
    } else {
        false
    }
}

fn budget_error(raw: &RawProjectSyntaxSnapshot, sources: &SourceMap) -> Option<Diagnostic> {
    if raw.files.len() > MAX_SOURCE_FILES {
        return Some(limit_error("syntax snapshot exceeds the source-file limit"));
    }
    if raw.diagnostics.len() > MAX_PROVIDER_DIAGNOSTICS {
        return Some(limit_error("provider diagnostics exceed the protocol-v3 limit"));
    }
    let mut source_bytes = 0_usize;
    for index in 0..sources.len() {
        let Ok(raw_id) = u32::try_from(index) else {
            return Some(limit_error("source file count cannot be represented by protocol v3"));
        };
        let Ok(id) = sources.verify_file_id(raw_id) else {
            return Some(limit_error("source map contains a non-canonical file id"));
        };
        let Some(source) = sources.source(id) else {
            return Some(limit_error("source map contains an unavailable file"));
        };
        if !checked_add(&mut source_bytes, source.text().len()) {
            return Some(limit_error("source byte count overflowed"));
        }
    }
    if source_bytes > MAX_AGGREGATE_SOURCE_BYTES {
        return Some(limit_error("source map exceeds the protocol-v3 aggregate byte limit"));
    }
    let mut counts = Counts::default();
    for file in &raw.files {
        if file.imports.len() > MAX_IMPORTS_PER_MODULE
            || file.functions.len() > MAX_FUNCTIONS_PER_MODULE
        {
            return Some(limit_error("one module exceeds a protocol-v3 declaration limit"));
        }
        if !checked_add(&mut counts.imports, file.imports.len())
            || !checked_add(&mut counts.functions, file.functions.len())
        {
            return Some(limit_error("syntax declaration count overflowed"));
        }
        for import in &file.imports {
            if import.bindings.is_empty()
                || import.bindings.len() > MAX_IMPORTED_NAMES_PER_DECLARATION
                || !checked_add(&mut counts.imported_names, import.bindings.len())
            {
                return Some(limit_error("imported-name count exceeds its protocol-v3 limit"));
            }
        }
        for function in &file.functions {
            let body = &function.body;
            if function.parameters.len() > MAX_PARAMETERS_PER_FUNCTION
                || body.blocks.is_empty()
                || body.blocks.len() > MAX_BLOCKS_PER_FUNCTION
                || body.statements.len() > MAX_STATEMENTS_PER_FUNCTION
                || body.expressions.len() > MAX_EXPRESSIONS_PER_FUNCTION
            {
                return Some(limit_error("one function exceeds a protocol-v3 collection limit"));
            }
            if body.blocks.iter().any(|block| block.statements.len() > MAX_STATEMENTS_PER_FUNCTION)
                || body.expressions.iter().any(|expression| {
                    matches!(
                        &expression.kind,
                        RawExpressionKind::Call { arguments, .. }
                            if arguments.len() > MAX_PARAMETERS_PER_FUNCTION
                    )
                })
            {
                return Some(limit_error("one arena edge list exceeds its protocol-v3 limit"));
            }
            if !checked_add(&mut counts.parameters, function.parameters.len())
                || !checked_add(&mut counts.blocks, body.blocks.len())
                || !checked_add(&mut counts.statements, body.statements.len())
                || !checked_add(&mut counts.expressions, body.expressions.len())
            {
                return Some(limit_error("syntax node count overflowed"));
            }
            let local_count = body
                .statements
                .iter()
                .filter(|statement| {
                    matches!(statement.kind, RawStatementKind::LocalDeclaration { .. })
                })
                .count();
            if local_count > MAX_LOCALS_PER_FUNCTION
                || !checked_add(&mut counts.locals, local_count)
            {
                return Some(limit_error("local declaration count exceeds its protocol-v3 limit"));
            }
        }
    }
    if exceeds_project_limits(&counts) {
        return Some(limit_error("syntax snapshot exceeds an aggregate protocol-v3 limit"));
    }
    None
}

/// Verifies every protocol-v3 claim against one exact authoritative source map.
///
/// # Errors
///
/// Returns bounded, deterministically ordered diagnostics when the snapshot exceeds a resource
/// budget, differs from the authoritative files or source text, or violates canonical arena,
/// span, ownership, or nesting invariants.
pub fn verify_snapshot(
    raw: RawProjectSyntaxSnapshot,
    sources: &SourceMap,
) -> Result<ProjectSyntaxSnapshot, Vec<Diagnostic>> {
    if let Some(error) = budget_error(&raw, sources) {
        return Err(vec![error]);
    }
    let mut errors = Errors::default();
    if raw.schema_version != PROTOCOL_VERSION {
        errors.protocol(None, "snapshot schema version is not exactly 3");
    }
    if raw.files.len() != sources.len() {
        errors.protocol(None, "snapshot file set is not complete");
    }
    let mut files = Vec::with_capacity(raw.files.len());
    for (position, file) in raw.files.into_iter().enumerate() {
        if let Some(file) = verify_file(file, position, sources, &mut errors) {
            files.push(file);
        }
    }
    let diagnostics = verify_diagnostics(raw.diagnostics, sources, &mut errors);
    if errors.items.is_empty() {
        Ok(ProjectSyntaxSnapshot {
            source_map_identity: sources.identity(),
            schema_version: PROTOCOL_VERSION,
            files,
            diagnostics,
        })
    } else {
        Err(errors.finish())
    }
}

fn verify_file(
    raw: RawSourceUnit,
    position: usize,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<SourceUnit> {
    let Ok(expected_raw) = u32::try_from(position) else {
        errors.protocol(None, "source position cannot be represented by protocol v3");
        return None;
    };
    if raw.id != expected_raw {
        errors.protocol(None, "source units are not in canonical dense file-id order");
        return None;
    }
    let Ok(id) = sources.verify_file_id(raw.id) else {
        errors.protocol(None, "source unit references an unknown file id");
        return None;
    };
    let Ok(path) = NormalizedSourcePath::new(raw.path) else {
        errors.protocol(None, "source unit path is not portable and normalized");
        return None;
    };
    if sources.source(id).is_none_or(|source| source.path() != &path) {
        errors
            .protocol(Some(path.as_str()), "source path does not match the authoritative file id");
        return None;
    }
    let mut imports = Vec::with_capacity(raw.imports.len());
    let mut previous_end = 0;
    for import in raw.imports {
        let import = verify_import(import, raw.id, &path, sources, errors)?;
        if import.span.start() < previous_end {
            errors.node(&path, "imports are not in source order");
        }
        previous_end = import.span.end();
        imports.push(import);
    }
    let mut functions = Vec::with_capacity(raw.functions.len());
    for function in raw.functions {
        let function = verify_function(function, raw.id, &path, sources, errors)?;
        if function.span.start() < previous_end {
            errors.node(&path, "top-level declarations are not in canonical source order");
        }
        previous_end = function.span.end();
        functions.push(function);
    }
    Some(SourceUnit { id, path, imports, functions })
}

fn verify_import(
    raw: RawImportSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<ImportSyntax> {
    let span = node_span(raw.span, file, path, sources, errors, "import")?;
    let import_span =
        token(raw.import_span, file, path, sources, errors, "import", "import keyword")?;
    let from_span = token(raw.from_span, file, path, sources, errors, "from", "from keyword")?;
    let semicolon_span =
        token(raw.semicolon_span, file, path, sources, errors, ";", "import semicolon")?;
    contains(span, import_span, path, errors, "import keyword");
    contains(span, from_span, path, errors, "from keyword");
    contains(span, semicolon_span, path, errors, "semicolon");
    let mut bindings = Vec::with_capacity(raw.bindings.len());
    let mut previous_end = import_span.end();
    for raw_binding in raw.bindings {
        let binding_span =
            node_span(raw_binding.span, file, path, sources, errors, "import binding")?;
        let imported =
            identifier(raw_binding.imported, file, path, sources, errors, "imported name")?;
        let local =
            identifier(raw_binding.local, file, path, sources, errors, "local import name")?;
        contains(binding_span, imported.span, path, errors, "imported name");
        contains(binding_span, local.span, path, errors, "local name");
        let as_span = raw_binding
            .as_span
            .and_then(|value| token(value, file, path, sources, errors, "as", "as keyword"));
        contains(span, binding_span, path, errors, "import binding");
        if let Some(as_span) = as_span {
            contains(binding_span, as_span, path, errors, "as keyword");
            require_order(imported.span, as_span, path, errors, "imported name and as keyword");
            require_order(as_span, local.span, path, errors, "as keyword and local name");
        }
        if as_span.is_none() && (imported.text != local.text || imported.span != local.span) {
            errors.node(path, "unaliased import must repeat the exact imported identifier");
        }
        if binding_span.start() < previous_end {
            errors.node(path, "import bindings are not in source order");
        }
        previous_end = binding_span.end();
        bindings.push(ImportBindingSyntax { span: binding_span, imported, local, as_span });
    }
    let specifier = module_specifier(raw.specifier, file, path, sources, errors)?;
    contains(span, specifier.token_span, path, errors, "module specifier");
    if from_span.start() < previous_end
        || specifier.token_span.start() < from_span.end()
        || semicolon_span.start() < specifier.token_span.end()
    {
        errors.node(path, "import tokens are not in source order");
    }
    Some(ImportSyntax { span, import_span, bindings, from_span, specifier, semicolon_span })
}

fn module_specifier(
    raw: RawModuleSpecifierSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<ModuleSpecifierSyntax> {
    if !valid_specifier(&raw.text) {
        errors.node(path, "module specifier is not canonical explicit relative .zry syntax");
        return None;
    }
    let token_span =
        node_span(raw.token_span, file, path, sources, errors, "module specifier token")?;
    let value_span =
        node_span(raw.value_span, file, path, sources, errors, "module specifier value")?;
    contains(token_span, value_span, path, errors, "module specifier value");
    require_text(value_span, &raw.text, path, sources, errors, "module specifier value");
    let quoted = format!("\"{}\"", raw.text);
    let single = format!("'{}'", raw.text);
    if !span_text(token_span, sources).is_some_and(|text| text == quoted || text == single) {
        errors.node(path, "module specifier token must be an unescaped quoted canonical value");
    }
    Some(ModuleSpecifierSyntax { text: raw.text, token_span, value_span })
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn valid_specifier(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_MODULE_SPECIFIER_BYTES
        || !value.is_ascii()
        || !(value.starts_with("./") || value.starts_with("../"))
        || !value.ends_with(".zry")
        || value.contains(['\\', '?', '#', '\0'])
        || value.contains("://")
    {
        return false;
    }
    let body = value.strip_prefix("./").unwrap_or_else(|| value.trim_start_matches("../"));
    !body.is_empty() && !value.split('/').any(str::is_empty)
}

fn verify_function(
    raw: RawFunctionSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<FunctionSyntax> {
    let span = node_span(raw.span, file, path, sources, errors, "function")?;
    let export_span = raw
        .export_span
        .and_then(|value| token(value, file, path, sources, errors, "export", "export keyword"));
    let function_span =
        token(raw.function_span, file, path, sources, errors, "function", "function keyword")?;
    let name = identifier(raw.name, file, path, sources, errors, "function name")?;
    let result_type = type_syntax(raw.result_type, file, path, sources, errors, "result type")?;
    let mut parameters = Vec::with_capacity(raw.parameters.len());
    let mut previous_end = name.span.end();
    for raw_parameter in raw.parameters {
        let parameter_span =
            node_span(raw_parameter.span, file, path, sources, errors, "parameter")?;
        let parameter_name =
            identifier(raw_parameter.name, file, path, sources, errors, "parameter name")?;
        let parameter_type =
            type_syntax(raw_parameter.type_syntax, file, path, sources, errors, "parameter type")?;
        contains(parameter_span, parameter_name.span, path, errors, "parameter name");
        contains(parameter_span, parameter_type.span, path, errors, "parameter type");
        contains(span, parameter_span, path, errors, "parameter");
        require_order(
            parameter_name.span,
            parameter_type.span,
            path,
            errors,
            "parameter name and type",
        );
        if parameter_span.start() < previous_end {
            errors.node(path, "parameters are not in source order");
        }
        previous_end = parameter_span.end();
        parameters.push(ParameterSyntax {
            span: parameter_span,
            name: parameter_name,
            type_syntax: parameter_type,
        });
    }
    let body_span = node_span(raw.body.span, file, path, sources, errors, "function body")?;
    for child in
        [export_span, Some(function_span), Some(name.span), Some(result_type.span), Some(body_span)]
            .into_iter()
            .flatten()
    {
        contains(span, child, path, errors, "function child");
    }
    if let Some(export_span) = export_span {
        require_order(export_span, function_span, path, errors, "export and function keywords");
    }
    require_order(function_span, name.span, path, errors, "function keyword and name");
    if let Some(first) = parameters.first() {
        require_order(name.span, first.span, path, errors, "function name and parameters");
    }
    if let Some(last) = parameters.last() {
        require_order(last.span, result_type.span, path, errors, "parameters and result type");
    } else {
        require_order(name.span, result_type.span, path, errors, "function name and result type");
    }
    require_order(result_type.span, body_span, path, errors, "result type and function body");
    let body = verify_body(raw.body, body_span, file, path, sources, errors)?;
    Some(FunctionSyntax { span, export_span, function_span, name, parameters, result_type, body })
}

fn type_syntax(
    raw: RawTypeSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) -> Option<TypeSyntax> {
    let span = node_span(raw.span, file, path, sources, errors, label)?;
    let kind = match raw.kind {
        RawTypeSyntaxKind::Missing => {
            if span.start() != span.end() {
                errors.node(path, "missing type must use an empty insertion span");
            }
            TypeSyntaxKind::Missing
        }
        RawTypeSyntaxKind::Named { name } => {
            if !bounded_text(&name, MAX_NAME_CHARACTERS) {
                errors.limit("type spelling exceeds the limit");
                return None;
            }
            require_text(span, &name, path, sources, errors, label);
            TypeSyntaxKind::Named { name }
        }
    };
    Some(TypeSyntax { span, kind })
}

fn identifier(
    raw: RawIdentifierSyntax,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) -> Option<IdentifierSyntax> {
    if !bounded_text(&raw.text, MAX_NAME_CHARACTERS) {
        errors.limit("identifier exceeds the limit");
        return None;
    }
    let span = node_span(raw.span, file, path, sources, errors, label)?;
    require_text(span, &raw.text, path, sources, errors, label);
    Some(IdentifierSyntax { text: raw.text, span })
}

fn verify_body(
    raw: RawFunctionBodySyntax,
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Option<FunctionBodySyntax> {
    if raw.root_block != 0 {
        errors.node(path, "root block id must be zero");
    }
    let mut expressions = Vec::with_capacity(raw.expressions.len());
    let mut expression_owners = vec![0_u32; raw.expressions.len()];
    let mut expression_depths = vec![0_u32; raw.expressions.len()];
    for (index, expression) in raw.expressions.into_iter().enumerate() {
        expressions.push(verify_expression(
            expression,
            index,
            body_span,
            file,
            path,
            sources,
            &expressions,
            &mut expression_owners,
            &mut expression_depths,
            errors,
        )?);
    }
    let mut statements = Vec::with_capacity(raw.statements.len());
    let mut block_owners = vec![0_u32; raw.blocks.len()];
    let mut expression_roots = Vec::new();
    for statement in raw.statements {
        statements.push(verify_statement(
            statement,
            body_span,
            file,
            path,
            sources,
            expressions.len(),
            &mut block_owners,
            &mut expression_owners,
            &mut expression_roots,
            errors,
        )?);
    }
    let mut statement_owners = vec![0_u32; statements.len()];
    let blocks = verify_blocks(
        raw.blocks,
        &statements,
        body_span,
        file,
        path,
        sources,
        &mut statement_owners,
        errors,
    )?;
    for statement in &statements {
        verify_statement_layout(statement, &expressions, &blocks, path, errors);
    }
    if block_owners.get_mut(0).is_some() {
        block_owners[0] = block_owners[0].saturating_add(1);
    }
    for (index, owners) in block_owners.into_iter().enumerate() {
        if owners != 1 {
            errors.node(path, &format!("block {index} has {owners} owners; expected one"));
        }
    }
    for (index, owners) in statement_owners.into_iter().enumerate() {
        if owners != 1 {
            errors.node(path, &format!("statement {index} has {owners} owners; expected one"));
        }
    }
    for (index, owners) in expression_owners.into_iter().enumerate() {
        if owners != 1 {
            errors.node(path, &format!("expression {index} has {owners} owners; expected one"));
        }
    }
    verify_canonical_graph(&blocks, &statements, &expressions, &expression_roots, path, errors);
    if let Some(root) = blocks.first() {
        contains(body_span, root.span, path, errors, "root block");
    }
    Some(FunctionBodySyntax {
        span: body_span,
        root_block: BlockId(raw.root_block),
        blocks,
        statements,
        expressions,
    })
}

#[allow(clippy::too_many_arguments)]
fn verify_blocks(
    raw_blocks: Vec<RawBlockSyntax>,
    statements: &[StatementSyntax],
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    statement_owners: &mut [u32],
    errors: &mut Errors,
) -> Option<Vec<BlockSyntax>> {
    let mut blocks = Vec::with_capacity(raw_blocks.len());
    for block in raw_blocks {
        let span = node_span(block.span, file, path, sources, errors, "block")?;
        contains(body_span, span, path, errors, "block");
        let open = token(block.open_brace_span, file, path, sources, errors, "{", "open brace")?;
        let close = token(block.close_brace_span, file, path, sources, errors, "}", "close brace")?;
        contains(span, open, path, errors, "open brace");
        contains(span, close, path, errors, "close brace");
        let mut ids = Vec::with_capacity(block.statements.len());
        let mut previous_end = open.end();
        for raw_id in block.statements {
            let Ok(index) = usize::try_from(raw_id) else {
                errors.node(path, "statement id does not fit host index");
                continue;
            };
            let Some(statement) = statements.get(index) else {
                errors.node(path, "block references an unknown statement");
                continue;
            };
            statement_owners[index] = statement_owners[index].saturating_add(1);
            contains(span, statement.span, path, errors, "statement");
            if statement.span.start() < previous_end {
                errors.node(path, "block statements are not in source order");
            }
            previous_end = statement.span.end();
            ids.push(StatementId(raw_id));
        }
        if close.start() < previous_end {
            errors.node(path, "block contents are not in source order");
        }
        blocks.push(BlockSyntax {
            span,
            open_brace_span: open,
            statements: ids,
            close_brace_span: close,
        });
    }
    Some(blocks)
}

fn verify_statement_layout(
    statement: &StatementSyntax,
    expressions: &[ExpressionSyntax],
    blocks: &[BlockSyntax],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    let expression_span = |id: ExpressionId| expressions.get(id.0 as usize).map(|value| value.span);
    let block_span = |id: BlockId| blocks.get(id.0 as usize).map(|value| value.span);
    let children = match &statement.kind {
        StatementKind::LocalDeclaration {
            keyword_span,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
            ..
        } => vec![
            Some(*keyword_span),
            Some(name.span),
            Some(type_syntax.span),
            Some(*equals_span),
            expression_span(*initializer),
            Some(*semicolon_span),
        ],
        StatementKind::Assignment { target, equals_span, value, semicolon_span } => vec![
            Some(target.span),
            Some(*equals_span),
            expression_span(*value),
            Some(*semicolon_span),
        ],
        StatementKind::Return { keyword_span, value, semicolon_span } => {
            vec![Some(*keyword_span), expression_span(*value), Some(*semicolon_span)]
        }
        StatementKind::Block { block } => vec![block_span(*block)],
        StatementKind::If {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            then_block,
            else_clause,
        } => {
            let mut spans = vec![
                Some(*keyword_span),
                Some(*open_paren_span),
                expression_span(*condition),
                Some(*close_paren_span),
                block_span(*then_block),
            ];
            if let Some(value) = else_clause {
                spans.push(Some(value.keyword_span));
                spans.push(block_span(value.block));
            }
            spans
        }
        StatementKind::While {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            body_block,
        } => vec![
            Some(*keyword_span),
            Some(*open_paren_span),
            expression_span(*condition),
            Some(*close_paren_span),
            block_span(*body_block),
        ],
    };
    let mut previous = None;
    for child in children.into_iter().flatten() {
        contains(statement.span, child, path, errors, "statement child");
        if let Some(before) = previous {
            require_order(before, child, path, errors, "statement children");
        }
        previous = Some(child);
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_expression(
    raw: RawExpressionSyntax,
    index: usize,
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    expressions: &[ExpressionSyntax],
    owners: &mut [u32],
    depths: &mut [u32],
    errors: &mut Errors,
) -> Option<ExpressionSyntax> {
    let span = node_span(raw.span, file, path, sources, errors, "expression")?;
    contains(body_span, span, path, errors, "expression");
    depths[index] = 1;
    let kind = match raw.kind {
        RawExpressionKind::Reference { name } => {
            let name = identifier(name, file, path, sources, errors, "reference")?;
            contains(span, name.span, path, errors, "reference name");
            ExpressionKind::Reference { name }
        }
        RawExpressionKind::BoolLiteral { value } => {
            require_text(
                span,
                if value { "true" } else { "false" },
                path,
                sources,
                errors,
                "boolean literal",
            );
            ExpressionKind::BoolLiteral { value }
        }
        RawExpressionKind::I32Literal { spelling } => {
            if !valid_integer(&spelling) {
                errors.node(path, "integer literal spelling is not canonical");
            }
            require_text(span, &spelling, path, sources, errors, "integer literal");
            ExpressionKind::I32Literal { spelling }
        }
        RawExpressionKind::Negation { operator_span, operand } => {
            let operator_span = token(operator_span, file, path, sources, errors, "-", "negation")?;
            let operand = expression_edge(operand, index, owners, depths, path, errors)?;
            let operand_span = expressions.get(operand.0 as usize)?.span;
            contains(span, operator_span, path, errors, "negation operator");
            contains(span, operand_span, path, errors, "negation operand");
            require_order(
                operator_span,
                operand_span,
                path,
                errors,
                "negation operator and operand",
            );
            ExpressionKind::Negation { operator_span, operand }
        }
        RawExpressionKind::Addition { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "+",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::Addition { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::Subtraction { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "-",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::Subtraction { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::Multiplication { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "*",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::Multiplication { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::Equal { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "===",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::Equal { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::NotEqual { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "!==",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::NotEqual { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::LessThan { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "<",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::LessThan { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::LessEqual { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                "<=",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::LessEqual { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::GreaterThan { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                ">",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::GreaterThan { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::GreaterEqual { operator_span, lhs, rhs } => {
            let (o, l, r) = binary_edges(
                span,
                operator_span,
                lhs,
                rhs,
                ">=",
                index,
                file,
                path,
                sources,
                expressions,
                owners,
                depths,
                errors,
            )?;
            ExpressionKind::GreaterEqual { operator_span: o, lhs: l, rhs: r }
        }
        RawExpressionKind::Call { callee, open_paren_span, arguments, close_paren_span } => {
            let callee = identifier(callee, file, path, sources, errors, "call callee")?;
            let open =
                token(open_paren_span, file, path, sources, errors, "(", "call open parenthesis")?;
            let close = token(
                close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "call close parenthesis",
            )?;
            let arguments = arguments
                .into_iter()
                .filter_map(|argument| {
                    expression_edge(argument, index, owners, depths, path, errors)
                })
                .collect::<Vec<_>>();
            contains(span, callee.span, path, errors, "call callee");
            contains(span, open, path, errors, "call open parenthesis");
            contains(span, close, path, errors, "call close parenthesis");
            require_order(callee.span, open, path, errors, "call callee and open parenthesis");
            let mut previous = open;
            for argument in &arguments {
                let argument_span = expressions.get(argument.0 as usize)?.span;
                contains(span, argument_span, path, errors, "call argument");
                require_order(previous, argument_span, path, errors, "call arguments");
                previous = argument_span;
            }
            require_order(previous, close, path, errors, "call close parenthesis");
            ExpressionKind::Call {
                callee,
                open_paren_span: open,
                arguments,
                close_paren_span: close,
            }
        }
    };
    if depths[index] > MAX_NESTING_DEPTH {
        errors.limit("expression nesting exceeds the protocol-v3 limit");
    }
    Some(ExpressionSyntax { span, kind })
}

#[allow(clippy::too_many_arguments)]
fn binary_edges(
    span: Span,
    operator_span: UntrustedSpan,
    lhs: u32,
    rhs: u32,
    expected: &str,
    index: usize,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    expressions: &[ExpressionSyntax],
    owners: &mut [u32],
    depths: &mut [u32],
    errors: &mut Errors,
) -> Option<(Span, ExpressionId, ExpressionId)> {
    let operator_span = token(operator_span, file, path, sources, errors, expected, "operator")?;
    let lhs = expression_edge(lhs, index, owners, depths, path, errors)?;
    let rhs = expression_edge(rhs, index, owners, depths, path, errors)?;
    let lhs_span = expressions.get(lhs.0 as usize)?.span;
    let rhs_span = expressions.get(rhs.0 as usize)?.span;
    contains(span, lhs_span, path, errors, "left operand");
    contains(span, operator_span, path, errors, "binary operator");
    contains(span, rhs_span, path, errors, "right operand");
    require_order(lhs_span, operator_span, path, errors, "left operand and operator");
    require_order(operator_span, rhs_span, path, errors, "operator and right operand");
    Some((operator_span, lhs, rhs))
}

fn expression_edge(
    raw_id: u32,
    index: usize,
    owners: &mut [u32],
    depths: &mut [u32],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) -> Option<ExpressionId> {
    let child = usize::try_from(raw_id).ok()?;
    if child >= index {
        errors.node(path, "expression edge is not canonical postorder");
        return None;
    }
    let owner = owners.get_mut(child)?;
    *owner = owner.saturating_add(1);
    let depth = depths.get(child).copied()?.saturating_add(1);
    depths[index] = depths[index].max(depth);
    Some(ExpressionId(raw_id))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_statement(
    raw: RawStatementSyntax,
    body_span: Span,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    expression_count: usize,
    block_owners: &mut [u32],
    expression_owners: &mut [u32],
    roots: &mut Vec<ExpressionId>,
    errors: &mut Errors,
) -> Option<StatementSyntax> {
    let span = node_span(raw.span, file, path, sources, errors, "statement")?;
    contains(body_span, span, path, errors, "statement");
    let kind = match raw.kind {
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable,
            name,
            type_syntax: raw_type,
            equals_span,
            initializer,
            semicolon_span,
        } => {
            let keyword = token(
                keyword_span,
                file,
                path,
                sources,
                errors,
                if mutable { "let" } else { "const" },
                "local keyword",
            )?;
            StatementKind::LocalDeclaration {
                keyword_span: keyword,
                mutable,
                name: identifier(name, file, path, sources, errors, "local name")?,
                type_syntax: type_syntax(raw_type, file, path, sources, errors, "local type")?,
                equals_span: token(
                    equals_span,
                    file,
                    path,
                    sources,
                    errors,
                    "=",
                    "initializer equals",
                )?,
                initializer: statement_expression(
                    initializer,
                    expression_count,
                    expression_owners,
                    roots,
                    path,
                    errors,
                )?,
                semicolon_span: token(
                    semicolon_span,
                    file,
                    path,
                    sources,
                    errors,
                    ";",
                    "local semicolon",
                )?,
            }
        }
        RawStatementKind::Assignment { target, equals_span, value, semicolon_span } => {
            StatementKind::Assignment {
                target: identifier(target, file, path, sources, errors, "assignment target")?,
                equals_span: token(
                    equals_span,
                    file,
                    path,
                    sources,
                    errors,
                    "=",
                    "assignment equals",
                )?,
                value: statement_expression(
                    value,
                    expression_count,
                    expression_owners,
                    roots,
                    path,
                    errors,
                )?,
                semicolon_span: token(
                    semicolon_span,
                    file,
                    path,
                    sources,
                    errors,
                    ";",
                    "assignment semicolon",
                )?,
            }
        }
        RawStatementKind::Return { keyword_span, value, semicolon_span } => StatementKind::Return {
            keyword_span: token(
                keyword_span,
                file,
                path,
                sources,
                errors,
                "return",
                "return keyword",
            )?,
            value: statement_expression(
                value,
                expression_count,
                expression_owners,
                roots,
                path,
                errors,
            )?,
            semicolon_span: token(
                semicolon_span,
                file,
                path,
                sources,
                errors,
                ";",
                "return semicolon",
            )?,
        },
        RawStatementKind::Block { block: raw_block } => {
            StatementKind::Block { block: statement_block(raw_block, block_owners, path, errors)? }
        }
        RawStatementKind::If {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            then_block,
            else_clause,
        } => StatementKind::If {
            keyword_span: token(keyword_span, file, path, sources, errors, "if", "if keyword")?,
            open_paren_span: token(
                open_paren_span,
                file,
                path,
                sources,
                errors,
                "(",
                "if open parenthesis",
            )?,
            condition: statement_expression(
                condition,
                expression_count,
                expression_owners,
                roots,
                path,
                errors,
            )?,
            close_paren_span: token(
                close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "if close parenthesis",
            )?,
            then_block: statement_block(then_block, block_owners, path, errors)?,
            else_clause: match else_clause {
                Some(value) => Some(ElseSyntax {
                    keyword_span: token(
                        value.keyword_span,
                        file,
                        path,
                        sources,
                        errors,
                        "else",
                        "else keyword",
                    )?,
                    block: statement_block(value.block, block_owners, path, errors)?,
                }),
                None => None,
            },
        },
        RawStatementKind::While {
            keyword_span,
            open_paren_span,
            condition,
            close_paren_span,
            body_block,
        } => StatementKind::While {
            keyword_span: token(
                keyword_span,
                file,
                path,
                sources,
                errors,
                "while",
                "while keyword",
            )?,
            open_paren_span: token(
                open_paren_span,
                file,
                path,
                sources,
                errors,
                "(",
                "while open parenthesis",
            )?,
            condition: statement_expression(
                condition,
                expression_count,
                expression_owners,
                roots,
                path,
                errors,
            )?,
            close_paren_span: token(
                close_paren_span,
                file,
                path,
                sources,
                errors,
                ")",
                "while close parenthesis",
            )?,
            body_block: statement_block(body_block, block_owners, path, errors)?,
        },
    };
    Some(StatementSyntax { span, kind })
}

fn statement_expression(
    raw_id: u32,
    expression_count: usize,
    owners: &mut [u32],
    roots: &mut Vec<ExpressionId>,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) -> Option<ExpressionId> {
    let index = usize::try_from(raw_id).ok()?;
    if index >= expression_count {
        errors.node(path, "statement references an unknown expression");
        return None;
    }
    owners[index] = owners[index].saturating_add(1);
    let id = ExpressionId(raw_id);
    roots.push(id);
    Some(id)
}

fn statement_block(
    raw_id: u32,
    owners: &mut [u32],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) -> Option<BlockId> {
    let index = usize::try_from(raw_id).ok()?;
    let Some(owner) = owners.get_mut(index) else {
        errors.node(path, "statement references an unknown block");
        return None;
    };
    *owner = owner.saturating_add(1);
    Some(BlockId(raw_id))
}

fn verify_canonical_graph(
    blocks: &[BlockSyntax],
    statements: &[StatementSyntax],
    expressions: &[ExpressionSyntax],
    roots: &[ExpressionId],
    path: &NormalizedSourcePath,
    errors: &mut Errors,
) {
    if blocks.is_empty() {
        errors.node(path, "function body does not contain its root block");
        return;
    }
    let mut expected_block = 0usize;
    let mut expected_statement = 0usize;
    let mut visited_blocks = vec![false; blocks.len()];
    let mut stack = vec![(BlockId(0), 0usize, 1u32, false)];
    while let Some((id, statement_offset, depth, entered)) = stack.pop() {
        if depth > MAX_NESTING_DEPTH {
            errors.limit("block nesting exceeds the protocol-v3 limit");
            break;
        }
        let Ok(index) = usize::try_from(id.0) else { continue };
        let Some(block) = blocks.get(index) else { continue };
        if !entered {
            if visited_blocks[index] {
                errors.node(path, "block graph contains a cycle or shared reachability");
                continue;
            }
            visited_blocks[index] = true;
            if index != expected_block {
                errors.node(path, "block arena is not canonical preorder");
            }
            expected_block = expected_block.saturating_add(1);
        }
        let Some(statement_id) = block.statements.get(statement_offset) else { continue };
        stack.push((id, statement_offset.saturating_add(1), depth, true));
        let Ok(si) = usize::try_from(statement_id.0) else { continue };
        let Some(statement) = statements.get(si) else { continue };
        if si != expected_statement {
            errors.node(path, "statement arena is not canonical preorder");
        }
        expected_statement = expected_statement.saturating_add(1);
        match &statement.kind {
            StatementKind::Block { block } => {
                stack.push((*block, 0, depth.saturating_add(1), false));
            }
            StatementKind::If { then_block, else_clause, .. } => {
                if let Some(value) = else_clause {
                    stack.push((value.block, 0, depth.saturating_add(1), false));
                }
                stack.push((*then_block, 0, depth.saturating_add(1), false));
            }
            StatementKind::While { body_block, .. } => {
                stack.push((*body_block, 0, depth.saturating_add(1), false));
            }
            _ => {}
        }
    }
    if visited_blocks.iter().any(|visited| !visited) {
        errors.node(path, "block graph contains an unreachable block");
    }
    let mut expected = 0usize;
    let mut emitted = vec![false; expressions.len()];
    for root in roots {
        let Ok(root) = usize::try_from(root.0) else { continue };
        let mut stack = vec![(root, false, 1u32)];
        while let Some((index, exit, depth)) = stack.pop() {
            let Some(expr) = expressions.get(index) else { continue };
            if depth > MAX_NESTING_DEPTH {
                errors.limit("expression nesting exceeds the protocol-v3 limit");
                break;
            }
            if exit {
                if !emitted[index] {
                    if index != expected {
                        errors.node(path, "expression arena is not canonical postorder");
                    }
                    emitted[index] = true;
                    expected = expected.saturating_add(1);
                }
                continue;
            }
            if emitted[index] {
                continue;
            }
            stack.push((index, true, depth));
            let children = expression_children(&expr.kind);
            for child in children.into_iter().rev() {
                if let Ok(child) = usize::try_from(child.0) {
                    stack.push((child, false, depth.saturating_add(1)));
                }
            }
        }
    }
}

fn expression_children(kind: &ExpressionKind) -> Vec<ExpressionId> {
    match kind {
        ExpressionKind::Negation { operand, .. } => vec![*operand],
        ExpressionKind::Addition { lhs, rhs, .. }
        | ExpressionKind::Subtraction { lhs, rhs, .. }
        | ExpressionKind::Multiplication { lhs, rhs, .. }
        | ExpressionKind::Equal { lhs, rhs, .. }
        | ExpressionKind::NotEqual { lhs, rhs, .. }
        | ExpressionKind::LessThan { lhs, rhs, .. }
        | ExpressionKind::LessEqual { lhs, rhs, .. }
        | ExpressionKind::GreaterThan { lhs, rhs, .. }
        | ExpressionKind::GreaterEqual { lhs, rhs, .. } => vec![*lhs, *rhs],
        ExpressionKind::Call { arguments, .. } => arguments.clone(),
        _ => Vec::new(),
    }
}

fn verify_diagnostics(
    raw: Vec<RawProviderDiagnostic>,
    sources: &SourceMap,
    errors: &mut Errors,
) -> Vec<Diagnostic> {
    let mut output = Vec::new();
    for value in raw {
        if !bounded_text(&value.code, MAX_NAME_CHARACTERS)
            || value.message.is_empty()
            || value.guidance.is_empty()
            || value.message.chars().count() > MAX_DIAGNOSTIC_TEXT_CHARACTERS
            || value.guidance.chars().count() > MAX_DIAGNOSTIC_TEXT_CHARACTERS
        {
            errors.limit("provider diagnostic text exceeds its limit");
            continue;
        }
        let span = match value.location {
            RawDiagnosticLocation::Global => None,
            RawDiagnosticLocation::Source { span } => {
                let Ok(span) = sources.verify_span(span) else {
                    errors.protocol(None, "provider diagnostic span is invalid");
                    continue;
                };
                Some(span)
            }
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

fn node_span(
    raw: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) -> Option<Span> {
    if raw.file != file {
        errors.node(path, &format!("{label} uses the wrong file id"));
        return None;
    }
    let Ok(span) = sources.verify_span(raw) else {
        errors.node(path, &format!("{label} span is invalid"));
        return None;
    };
    Some(span)
}
fn token(
    raw: UntrustedSpan,
    file: u32,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    text: &str,
    label: &str,
) -> Option<Span> {
    let span = node_span(raw, file, path, sources, errors, label)?;
    require_text(span, text, path, sources, errors, label);
    Some(span)
}
fn span_text(span: Span, sources: &SourceMap) -> Option<&str> {
    let resolved = sources.resolve(span).ok()?;
    let start = usize::try_from(span.start()).ok()?;
    let end = usize::try_from(span.end()).ok()?;
    resolved.source().text().get(start..end)
}
fn require_text(
    span: Span,
    expected: &str,
    path: &NormalizedSourcePath,
    sources: &SourceMap,
    errors: &mut Errors,
    label: &str,
) {
    if span_text(span, sources) != Some(expected) {
        errors.node(path, &format!("{label} spelling disagrees with authoritative source"));
    }
}
fn contains(
    parent: Span,
    child: Span,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    if parent.file() != child.file() || child.start() < parent.start() || child.end() > parent.end()
    {
        errors.node(path, &format!("{label} is outside its owner span"));
    }
}
fn require_order(
    before: Span,
    after: Span,
    path: &NormalizedSourcePath,
    errors: &mut Errors,
    label: &str,
) {
    if before.file() != after.file() || after.start() < before.end() {
        errors.node(path, &format!("{label} are not in source order"));
    }
}
fn bounded_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control) && value.chars().count() <= limit
}
fn valid_integer(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_LITERAL_BYTES || !value.is_ascii() {
        return false;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && (digits == "0"
            || (!digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())))
        && value != "-0"
}

#[derive(Default)]
struct Errors {
    items: Vec<Diagnostic>,
    truncated: bool,
}
impl Errors {
    fn push(&mut self, d: Diagnostic) {
        if self.items.len() < MAX_VALIDATION_ERRORS - 1 {
            self.items.push(d);
        } else {
            self.truncated = true;
        }
    }
    fn protocol(&mut self, path: Option<&str>, message: &str) {
        self.push(Diagnostic::error(
            "ZRYNA-Y2001",
            path.map(str::to_owned),
            message,
            "return the exact protocol-v3 contract",
        ));
    }
    fn node(&mut self, path: &NormalizedSourcePath, message: &str) {
        self.push(Diagnostic::error(
            "ZRYNA-Y2002",
            Some(path.as_str().to_owned()),
            message,
            "return source-faithful canonical syntax",
        ));
    }
    fn limit(&mut self, message: &str) {
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
    Diagnostic::error("ZRYNA-F1201", None, message.into(), "reduce the bounded protocol-v3 input")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use zryna_source::SourceFileInput;

    const SOURCE: &str = "export function one(): i32 { return 1; }";
    fn span(start: usize, end: usize) -> UntrustedSpan {
        UntrustedSpan {
            file: 0,
            start: u32::try_from(start).unwrap(),
            end: u32::try_from(end).unwrap(),
        }
    }
    fn raw() -> RawProjectSyntaxSnapshot {
        let export = SOURCE.find("export").unwrap();
        let function = SOURCE.find("function").unwrap();
        let name = SOURCE.find("one").unwrap();
        let ty = SOURCE.find("i32").unwrap();
        let open = SOURCE.find('{').unwrap();
        let ret = SOURCE.find("return").unwrap();
        let literal = SOURCE.find('1').unwrap();
        let semi = SOURCE.find(';').unwrap();
        let close = SOURCE.rfind('}').unwrap();
        RawProjectSyntaxSnapshot {
            schema_version: 3,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                functions: vec![RawFunctionSyntax {
                    span: span(0, SOURCE.len()),
                    export_span: Some(span(export, export + 6)),
                    function_span: span(function, function + 8),
                    name: RawIdentifierSyntax { text: "one".into(), span: span(name, name + 3) },
                    parameters: vec![],
                    result_type: RawTypeSyntax {
                        span: span(ty, ty + 3),
                        kind: RawTypeSyntaxKind::Named { name: "i32".into() },
                    },
                    body: RawFunctionBodySyntax {
                        span: span(open, close + 1),
                        root_block: 0,
                        blocks: vec![RawBlockSyntax {
                            span: span(open, close + 1),
                            open_brace_span: span(open, open + 1),
                            statements: vec![0],
                            close_brace_span: span(close, close + 1),
                        }],
                        statements: vec![RawStatementSyntax {
                            span: span(ret, semi + 1),
                            kind: RawStatementKind::Return {
                                keyword_span: span(ret, ret + 6),
                                value: 0,
                                semicolon_span: span(semi, semi + 1),
                            },
                        }],
                        expressions: vec![RawExpressionSyntax {
                            span: span(literal, literal + 1),
                            kind: RawExpressionKind::I32Literal { spelling: "1".into() },
                        }],
                    },
                }],
            }],
            diagnostics: vec![],
        }
    }
    fn sources() -> SourceMap {
        SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text: SOURCE.into() }])
            .unwrap()
    }
    fn adapter_fixture() -> (RawProjectSyntaxSnapshot, SourceMap) {
        const REQUEST: &str =
            include_str!("../../../tests/fixtures/typescript-adapter-v3-request.json");
        const RESULT: &str =
            include_str!("../../../tests/fixtures/typescript-adapter-v3-result.json");
        let request: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
        let files = request["params"]["files"].as_array().unwrap();
        let sources = SourceMap::build(
            files
                .iter()
                .map(|file| SourceFileInput {
                    path: file["path"].as_str().unwrap().to_owned(),
                    text: file["text"].as_str().unwrap().to_owned(),
                })
                .collect(),
        )
        .unwrap();
        (decode_snapshot(RESULT.as_bytes()).unwrap(), sources)
    }
    fn rejection_text(raw: RawProjectSyntaxSnapshot, sources: &SourceMap) -> String {
        verify_snapshot(raw, sources)
            .expect_err("adversarial snapshot must fail")
            .into_iter()
            .map(|diagnostic| diagnostic.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn negation_snapshot(count: usize) -> (RawProjectSyntaxSnapshot, SourceMap) {
        let prefix = "export function one(): i32 { return ";
        let suffix = "; }";
        let text = format!("{prefix}{}1{suffix}", "-".repeat(count));
        let function = text.find("function").unwrap();
        let name = text.find("one").unwrap();
        let ty = text.find("i32").unwrap();
        let open = text.find('{').unwrap();
        let ret = text.find("return").unwrap();
        let literal = prefix.len() + count;
        let semi = text.find(';').unwrap();
        let close = text.rfind('}').unwrap();
        let mut expressions = vec![RawExpressionSyntax {
            span: span(literal, literal + 1),
            kind: RawExpressionKind::I32Literal { spelling: "1".into() },
        }];
        for offset in 0..count {
            let operator = literal - offset - 1;
            expressions.push(RawExpressionSyntax {
                span: span(operator, literal + 1),
                kind: RawExpressionKind::Negation {
                    operator_span: span(operator, operator + 1),
                    operand: u32::try_from(offset).unwrap(),
                },
            });
        }
        let snapshot = RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                functions: vec![RawFunctionSyntax {
                    span: span(0, text.len()),
                    export_span: Some(span(0, 6)),
                    function_span: span(function, function + 8),
                    name: RawIdentifierSyntax { text: "one".into(), span: span(name, name + 3) },
                    parameters: vec![],
                    result_type: RawTypeSyntax {
                        span: span(ty, ty + 3),
                        kind: RawTypeSyntaxKind::Named { name: "i32".into() },
                    },
                    body: RawFunctionBodySyntax {
                        span: span(open, close + 1),
                        root_block: 0,
                        blocks: vec![RawBlockSyntax {
                            span: span(open, close + 1),
                            open_brace_span: span(open, open + 1),
                            statements: vec![0],
                            close_brace_span: span(close, close + 1),
                        }],
                        statements: vec![RawStatementSyntax {
                            span: span(ret, semi + 1),
                            kind: RawStatementKind::Return {
                                keyword_span: span(ret, ret + 6),
                                value: u32::try_from(count).unwrap(),
                                semicolon_span: span(semi, semi + 1),
                            },
                        }],
                        expressions,
                    },
                }],
            }],
            diagnostics: vec![],
        };
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text }]).unwrap();
        (snapshot, sources)
    }

    fn nested_block_snapshot(count: usize) -> (RawProjectSyntaxSnapshot, SourceMap) {
        assert!(count > 0);
        let prefix = "export function one(): i32 ";
        let text = format!("{prefix}{}{}", "{".repeat(count), "}".repeat(count));
        let function = text.find("function").unwrap();
        let name = text.find("one").unwrap();
        let ty = text.find("i32").unwrap();
        let mut blocks = Vec::with_capacity(count);
        let mut statements = Vec::with_capacity(count.saturating_sub(1));
        for index in 0..count {
            let open = prefix.len() + index;
            let close = prefix.len() + (count * 2) - index - 1;
            blocks.push(RawBlockSyntax {
                span: span(open, close + 1),
                open_brace_span: span(open, open + 1),
                statements: if index + 1 < count {
                    vec![u32::try_from(index).unwrap()]
                } else {
                    vec![]
                },
                close_brace_span: span(close, close + 1),
            });
            if index + 1 < count {
                statements.push(RawStatementSyntax {
                    span: span(open + 1, close),
                    kind: RawStatementKind::Block { block: u32::try_from(index + 1).unwrap() },
                });
            }
        }
        let snapshot = RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: vec![],
                functions: vec![RawFunctionSyntax {
                    span: span(0, text.len()),
                    export_span: Some(span(0, 6)),
                    function_span: span(function, function + 8),
                    name: RawIdentifierSyntax { text: "one".into(), span: span(name, name + 3) },
                    parameters: vec![],
                    result_type: RawTypeSyntax {
                        span: span(ty, ty + 3),
                        kind: RawTypeSyntaxKind::Named { name: "i32".into() },
                    },
                    body: RawFunctionBodySyntax {
                        span: span(prefix.len(), text.len()),
                        root_block: 0,
                        blocks,
                        statements,
                        expressions: vec![],
                    },
                }],
            }],
            diagnostics: vec![],
        };
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text }]).unwrap();
        (snapshot, sources)
    }
    #[test]
    fn valid_snapshot_is_opaque_and_source_bound() {
        let source = sources();
        let snapshot = verify_snapshot(raw(), &source).unwrap();
        assert!(snapshot.is_bound_to(&source));
        assert_eq!(snapshot.files()[0].functions().len(), 1);
    }
    #[test]
    fn rejects_wrong_version_unknown_fields_and_shared_roots() {
        let source = sources();
        let mut wrong = raw();
        wrong.schema_version = 2;
        assert!(verify_snapshot(wrong, &source).is_err());
        let mut shared = raw();
        let duplicate = shared.files[0].functions[0].body.statements[0].clone();
        shared.files[0].functions[0].body.statements.push(duplicate);
        shared.files[0].functions[0].body.blocks[0].statements.push(1);
        assert!(verify_snapshot(shared, &source).is_err());
        let mut value = serde_json::to_value(raw()).unwrap();
        value.as_object_mut().unwrap().insert("unknown".into(), serde_json::Value::Bool(true));
        assert!(decode_snapshot(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn response_limit_fails_before_decode() {
        let mut bytes = vec![b' '; MAX_RESPONSE_BYTES];
        assert!(matches!(decode_snapshot(&bytes), Err(SyntaxDecodeError::InvalidSnapshot)));
        bytes.push(b' ');
        assert!(matches!(decode_snapshot(&bytes), Err(SyntaxDecodeError::ResponseTooLarge { .. })));
    }

    #[test]
    fn invalid_paths_and_unknown_blocks_never_yield_partial_snapshots() {
        let source = sources();
        let mut invalid_path = raw();
        invalid_path.files[0].path = "../main.zry".into();
        assert!(verify_snapshot(invalid_path, &source).is_err());

        let mut unknown_block = raw();
        unknown_block.files[0].functions[0].body.statements[0].kind =
            RawStatementKind::Block { block: 1 };
        assert!(verify_snapshot(unknown_block, &source).is_err());
    }

    #[test]
    fn empty_cyclic_and_unreachable_block_graphs_fail_closed() {
        let source = sources();

        let mut empty = raw();
        empty.files[0].functions[0].body.blocks.clear();
        empty.files[0].functions[0].body.statements.clear();
        empty.files[0].functions[0].body.expressions.clear();
        assert!(verify_snapshot(empty, &source).is_err());

        let mut cycle = raw();
        cycle.files[0].functions[0].body.statements[0].kind = RawStatementKind::Block { block: 0 };
        assert!(verify_snapshot(cycle, &source).is_err());

        let mut unreachable = raw();
        unreachable.files[0].functions[0].body.blocks.push(RawBlockSyntax {
            span: span(0, SOURCE.len()),
            open_brace_span: span(SOURCE.find('{').unwrap(), SOURCE.find('{').unwrap() + 1),
            statements: vec![],
            close_brace_span: span(SOURCE.rfind('}').unwrap(), SOURCE.rfind('}').unwrap() + 1),
        });
        assert!(verify_snapshot(unreachable, &source).is_err());
    }

    #[test]
    fn programmatic_edge_lists_obey_the_same_bounds_as_json() {
        let source = sources();
        let mut block_edges = raw();
        block_edges.files[0].functions[0].body.blocks[0].statements =
            vec![0; MAX_STATEMENTS_PER_FUNCTION + 1];
        assert!(verify_snapshot(block_edges, &source).is_err());

        let mut call_arguments = raw();
        call_arguments.files[0].functions[0].body.expressions[0].kind = RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "one".into(),
                span: span(SOURCE.find("one").unwrap(), SOURCE.find("one").unwrap() + 3),
            },
            open_paren_span: span(0, 1),
            arguments: vec![0; MAX_PARAMETERS_PER_FUNCTION + 1],
            close_paren_span: span(0, 1),
        };
        assert!(verify_snapshot(call_arguments, &source).is_err());
    }

    #[test]
    fn every_collection_budget_accepts_the_limit_and_rejects_one_more() {
        let source = sources();
        let (adapter, _) = adapter_fixture();
        let import = adapter.files[0].imports[0].clone();
        let binding = import.bindings[0].clone();
        let parameter = adapter.files[0].functions[0].parameters[0].clone();

        let mut files = raw();
        let empty_file = RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: vec![],
            functions: vec![],
        };
        files.files = vec![empty_file.clone(); MAX_SOURCE_FILES];
        assert!(budget_error(&files, &source).is_none());
        files.files.push(empty_file);
        assert!(budget_error(&files, &source).is_some());

        let diagnostic = RawProviderDiagnostic {
            code: "TS1".into(),
            severity: Severity::Warning,
            location: RawDiagnosticLocation::Global,
            message: "problem".into(),
            guidance: "fix it".into(),
        };
        let mut diagnostics = raw();
        diagnostics.diagnostics = vec![diagnostic.clone(); MAX_PROVIDER_DIAGNOSTICS];
        assert!(budget_error(&diagnostics, &source).is_none());
        diagnostics.diagnostics.push(diagnostic);
        assert!(budget_error(&diagnostics, &source).is_some());

        let mut imports = raw();
        imports.files[0].imports = vec![import.clone(); MAX_IMPORTS_PER_MODULE];
        assert!(budget_error(&imports, &source).is_none());
        imports.files[0].imports.push(import.clone());
        assert!(budget_error(&imports, &source).is_some());

        let mut bindings = raw();
        let mut declaration = import;
        declaration.bindings = vec![binding.clone(); MAX_IMPORTED_NAMES_PER_DECLARATION];
        bindings.files[0].imports.push(declaration.clone());
        assert!(budget_error(&bindings, &source).is_none());
        declaration.bindings.push(binding);
        bindings.files[0].imports[0] = declaration;
        assert!(budget_error(&bindings, &source).is_some());

        let function = raw().files[0].functions[0].clone();
        let mut functions = raw();
        functions.files[0].functions = vec![function.clone(); MAX_FUNCTIONS_PER_MODULE];
        assert!(budget_error(&functions, &source).is_none());
        functions.files[0].functions.push(function);
        assert!(budget_error(&functions, &source).is_some());

        let mut parameters = raw();
        parameters.files[0].functions[0].parameters =
            vec![parameter.clone(); MAX_PARAMETERS_PER_FUNCTION];
        assert!(budget_error(&parameters, &source).is_none());
        parameters.files[0].functions[0].parameters.push(parameter);
        assert!(budget_error(&parameters, &source).is_some());

        let block = raw().files[0].functions[0].body.blocks[0].clone();
        let mut blocks = raw();
        blocks.files[0].functions[0].body.blocks = vec![block.clone(); MAX_BLOCKS_PER_FUNCTION];
        assert!(budget_error(&blocks, &source).is_none());
        blocks.files[0].functions[0].body.blocks.push(block);
        assert!(budget_error(&blocks, &source).is_some());

        let statement = raw().files[0].functions[0].body.statements[0].clone();
        let mut statements = raw();
        statements.files[0].functions[0].body.statements =
            vec![statement.clone(); MAX_STATEMENTS_PER_FUNCTION];
        assert!(budget_error(&statements, &source).is_none());
        statements.files[0].functions[0].body.statements.push(statement.clone());
        assert!(budget_error(&statements, &source).is_some());

        let expression = raw().files[0].functions[0].body.expressions[0].clone();
        let mut expressions = raw();
        expressions.files[0].functions[0].body.expressions =
            vec![expression.clone(); MAX_EXPRESSIONS_PER_FUNCTION];
        assert!(budget_error(&expressions, &source).is_none());
        expressions.files[0].functions[0].body.expressions.push(expression);
        assert!(budget_error(&expressions, &source).is_some());

        let mut local = statement;
        local.kind = RawStatementKind::LocalDeclaration {
            keyword_span: span(0, 1),
            mutable: false,
            name: RawIdentifierSyntax { text: "x".into(), span: span(0, 1) },
            type_syntax: RawTypeSyntax {
                span: span(0, 1),
                kind: RawTypeSyntaxKind::Named { name: "i32".into() },
            },
            equals_span: span(0, 1),
            initializer: 0,
            semicolon_span: span(0, 1),
        };
        let mut locals = raw();
        locals.files[0].functions[0].body.statements = vec![local.clone(); MAX_LOCALS_PER_FUNCTION];
        assert!(budget_error(&locals, &source).is_none());
        locals.files[0].functions[0].body.statements.push(local);
        assert!(budget_error(&locals, &source).is_some());
    }

    #[test]
    fn aggregate_count_and_text_boundaries_are_exact() {
        let exact = Counts {
            imports: MAX_IMPORTS_PER_PROJECT,
            imported_names: MAX_IMPORTED_NAMES_PER_PROJECT,
            functions: MAX_FUNCTIONS_PER_PROJECT,
            parameters: MAX_PARAMETERS_PER_PROJECT,
            blocks: MAX_BLOCKS_PER_PROJECT,
            statements: MAX_STATEMENTS_PER_PROJECT,
            expressions: MAX_EXPRESSIONS_PER_PROJECT,
            locals: MAX_LOCALS_PER_PROJECT,
        };
        assert!(!exceeds_project_limits(&exact));
        for over in [
            Counts { imports: MAX_IMPORTS_PER_PROJECT + 1, ..Counts::default() },
            Counts { imported_names: MAX_IMPORTED_NAMES_PER_PROJECT + 1, ..Counts::default() },
            Counts { functions: MAX_FUNCTIONS_PER_PROJECT + 1, ..Counts::default() },
            Counts { parameters: MAX_PARAMETERS_PER_PROJECT + 1, ..Counts::default() },
            Counts { blocks: MAX_BLOCKS_PER_PROJECT + 1, ..Counts::default() },
            Counts { statements: MAX_STATEMENTS_PER_PROJECT + 1, ..Counts::default() },
            Counts { expressions: MAX_EXPRESSIONS_PER_PROJECT + 1, ..Counts::default() },
            Counts { locals: MAX_LOCALS_PER_PROJECT + 1, ..Counts::default() },
        ] {
            assert!(exceeds_project_limits(&over));
        }

        let specifier = format!("./{}.zry", "a".repeat(MAX_MODULE_SPECIFIER_BYTES - 6));
        assert_eq!(specifier.len(), MAX_MODULE_SPECIFIER_BYTES);
        assert!(valid_specifier(&specifier));
        let long_specifier = format!("./{}.zry", "a".repeat(MAX_MODULE_SPECIFIER_BYTES - 6 + 1));
        assert_eq!(long_specifier.len(), MAX_MODULE_SPECIFIER_BYTES + 1);
        assert!(!valid_specifier(&long_specifier));

        assert!(bounded_text(&"x".repeat(MAX_NAME_CHARACTERS), MAX_NAME_CHARACTERS));
        assert!(!bounded_text(&"x".repeat(MAX_NAME_CHARACTERS + 1), MAX_NAME_CHARACTERS));
        assert!(valid_integer(&"1".repeat(MAX_LITERAL_BYTES)));
        assert!(!valid_integer(&"1".repeat(MAX_LITERAL_BYTES + 1)));
    }

    #[test]
    fn aggregate_source_budget_accepts_the_limit_and_rejects_one_more_byte() {
        let chunk = "x".repeat(2 * 1_024 * 1_024);
        let exact = SourceMap::build(
            (0..4)
                .map(|index| SourceFileInput {
                    path: format!("src/f{index}.zry"),
                    text: chunk.clone(),
                })
                .collect(),
        )
        .unwrap();
        assert!(budget_error(&raw(), &exact).is_none());

        let over = SourceMap::build(
            (0..4)
                .map(|index| SourceFileInput {
                    path: format!("src/f{index}.zry"),
                    text: chunk.clone(),
                })
                .chain(std::iter::once(SourceFileInput {
                    path: "src/extra.zry".into(),
                    text: "x".into(),
                }))
                .collect(),
        )
        .unwrap();
        assert!(budget_error(&raw(), &over).is_some());
    }

    #[test]
    fn iterative_nesting_checks_accept_exact_limits_and_are_stack_safe() {
        let (exact_expression, exact_expression_sources) =
            negation_snapshot(usize::try_from(MAX_NESTING_DEPTH - 1).unwrap());
        assert!(verify_snapshot(exact_expression, &exact_expression_sources).is_ok());
        let (deep_expression, deep_expression_sources) =
            negation_snapshot(usize::try_from(MAX_NESTING_DEPTH).unwrap());
        assert!(
            rejection_text(deep_expression, &deep_expression_sources)
                .contains("expression nesting exceeds")
        );

        let (exact_blocks, exact_block_sources) =
            nested_block_snapshot(usize::try_from(MAX_NESTING_DEPTH).unwrap());
        assert!(verify_snapshot(exact_blocks, &exact_block_sources).is_ok());
        let (deep_blocks, deep_block_sources) =
            nested_block_snapshot(usize::try_from(MAX_NESTING_DEPTH + 1).unwrap());
        assert!(rejection_text(deep_blocks, &deep_block_sources).contains("block nesting exceeds"));

        let (max_expression_arena, max_expression_sources) =
            negation_snapshot(MAX_EXPRESSIONS_PER_FUNCTION - 1);
        assert!(verify_snapshot(max_expression_arena, &max_expression_sources).is_err());
        let (max_block_arena, max_block_sources) = nested_block_snapshot(MAX_BLOCKS_PER_FUNCTION);
        assert!(verify_snapshot(max_block_arena, &max_block_sources).is_err());
    }

    #[test]
    fn validation_error_budget_reserves_one_terminal_diagnostic() {
        let path = NormalizedSourcePath::new("src/main.zry").unwrap();
        let mut exact = Errors::default();
        for _ in 0..(MAX_VALIDATION_ERRORS - 1) {
            exact.node(&path, "bad claim");
        }
        assert_eq!(exact.finish().len(), MAX_VALIDATION_ERRORS - 1);

        let mut over = Errors::default();
        for _ in 0..MAX_VALIDATION_ERRORS {
            over.node(&path, "bad claim");
        }
        let errors = over.finish();
        assert_eq!(errors.len(), MAX_VALIDATION_ERRORS);
        assert!(errors.last().unwrap().to_string().contains("validation diagnostics exceeded"));
    }

    #[test]
    fn nested_spans_and_source_order_are_authoritative() {
        let (base, sources) = adapter_fixture();

        let mut import_owner = base.clone();
        import_owner.files[0].imports[0].span.end =
            import_owner.files[0].imports[0].bindings[0].span.start;
        assert!(rejection_text(import_owner, &sources).contains("outside its owner span"));

        let mut alias_owner = base.clone();
        alias_owner.files[0].imports[0].bindings[0].span.end =
            alias_owner.files[0].imports[0].bindings[0].as_span.unwrap().start;
        assert!(rejection_text(alias_owner, &sources).contains("outside its owner span"));

        let mut parameter_owner = base.clone();
        parameter_owner.files[0].functions[0].parameters[0].span.end =
            parameter_owner.files[0].functions[0].parameters[0].name.span.end;
        assert!(rejection_text(parameter_owner, &sources).contains("outside its owner span"));

        let mut reference_owner = base.clone();
        reference_owner.files[0].functions[1].body.expressions[1].span.start += 1;
        assert!(rejection_text(reference_owner, &sources).contains("outside its owner span"));

        let mut binary_owner = base.clone();
        let operator_start = match &binary_owner.files[0].functions[1].body.expressions[4].kind {
            RawExpressionKind::Multiplication { operator_span, .. } => operator_span.start,
            _ => panic!("fixture expression must be multiplication"),
        };
        binary_owner.files[0].functions[1].body.expressions[4].span.start = operator_start + 1;
        assert!(rejection_text(binary_owner, &sources).contains("outside its owner span"));

        let mut call_order = base.clone();
        let RawExpressionKind::Call { arguments, .. } =
            &mut call_order.files[0].functions[1].body.expressions[8].kind
        else {
            panic!("fixture expression must be a call");
        };
        arguments.swap(0, 1);
        assert!(rejection_text(call_order, &sources).contains("not in source order"));

        let mut statement_owner = base.clone();
        statement_owner.files[0].functions[1].body.statements[2].span.end =
            statement_owner.files[0].functions[1].body.blocks[1].span.start;
        assert!(rejection_text(statement_owner, &sources).contains("outside its owner span"));

        let mut block_order = base;
        block_order.files[0].functions[1].body.blocks[0].close_brace_span =
            block_order.files[0].functions[1].body.blocks[1].close_brace_span;
        assert!(rejection_text(block_order, &sources).contains("not in source order"));
    }

    #[test]
    fn provider_diagnostic_text_is_non_empty_and_bounded() {
        let source = sources();
        for (message, guidance) in [("", "fix"), ("problem", "")] {
            let mut value = raw();
            value.diagnostics.push(RawProviderDiagnostic {
                code: "TS1".into(),
                severity: Severity::Error,
                location: RawDiagnosticLocation::Global,
                message: message.into(),
                guidance: guidance.into(),
            });
            assert!(rejection_text(value, &source).contains("provider diagnostic text exceeds"));
        }
        let mut exact = raw();
        exact.diagnostics.push(RawProviderDiagnostic {
            code: "TS1".into(),
            severity: Severity::Warning,
            location: RawDiagnosticLocation::Global,
            message: "m".repeat(MAX_DIAGNOSTIC_TEXT_CHARACTERS),
            guidance: "g".repeat(MAX_DIAGNOSTIC_TEXT_CHARACTERS),
        });
        assert!(verify_snapshot(exact, &source).is_ok());

        let mut plus_one = raw();
        plus_one.diagnostics.push(RawProviderDiagnostic {
            code: "TS1".into(),
            severity: Severity::Warning,
            location: RawDiagnosticLocation::Global,
            message: "m".repeat(MAX_DIAGNOSTIC_TEXT_CHARACTERS + 1),
            guidance: "g".into(),
        });
        assert!(rejection_text(plus_one, &source).contains("provider diagnostic text exceeds"));
    }

    #[test]
    fn checked_in_schema_and_golden_fixture_match_the_runtime_contract() {
        const GOLDEN: &str = include_str!("../../../tests/fixtures/syntax-v3-valid.json");
        const SCHEMA: &str = include_str!("../../../schemas/zryna-syntax-v3.schema.json");
        let source = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".into(),
            text: "export function yes(): bool { return true; }".into(),
        }])
        .unwrap();
        let raw = decode_snapshot(GOLDEN.as_bytes()).expect("golden fixture must decode");
        let verified = verify_snapshot(raw, &source).expect("golden fixture must verify");
        assert_eq!(verified.schema_version(), PROTOCOL_VERSION);
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("checked-in schema must be valid JSON");
        assert_eq!(schema["properties"]["schema_version"]["const"], PROTOCOL_VERSION);
        assert_eq!(
            schema["$defs"]["body"]["properties"]["expressions"]["maxItems"],
            MAX_EXPRESSIONS_PER_FUNCTION,
        );
    }

    #[test]
    fn typescript_v3_adapter_fixture_passes_the_authoritative_verifier() {
        const REQUEST: &str =
            include_str!("../../../tests/fixtures/typescript-adapter-v3-request.json");
        const RESULT: &str =
            include_str!("../../../tests/fixtures/typescript-adapter-v3-result.json");
        let request: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
        let files = request["params"]["files"].as_array().unwrap();
        let sources = SourceMap::build(
            files
                .iter()
                .map(|file| SourceFileInput {
                    path: file["path"].as_str().unwrap().to_owned(),
                    text: file["text"].as_str().unwrap().to_owned(),
                })
                .collect(),
        )
        .unwrap();
        let raw = decode_snapshot(RESULT.as_bytes()).expect("adapter fixture must decode");
        let snapshot = verify_snapshot(raw, &sources).expect("adapter fixture must verify");
        assert!(snapshot.is_bound_to(&sources));
        assert_eq!(snapshot.files().len(), 2);
    }
}
