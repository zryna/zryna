//! Immutable compiler-owned definition facts for the first semantic query slice.

use zryna_diagnostics::Diagnostic;
use zryna_source::{FileId, SourceMap, SourceMapIdentity, Span};
use zryna_syntax::v2::ExpressionKind;

use crate::{SemanticInput, lower};

/// Deterministic bytes charged for one retained definition record.
pub const DEFINITION_RECORD_CACHE_BYTES: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DefinitionRecord {
    selection: Span,
    declaration: Span,
}

/// Opaque, immutable definition authority derived by successful semantic analysis.
#[derive(Clone, Debug)]
pub struct DefinitionIndex {
    source_identity: SourceMapIdentity,
    records: Vec<DefinitionRecord>,
}

/// Bounded definition lookup outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionLookup {
    /// The smallest containing identifier resolves to this declaration-name span.
    Found(Span),
    /// No supported semantic symbol occupies the requested byte.
    Absent,
    /// The caller's logical work limit was exhausted before lookup completed.
    OverBudget,
}

impl DefinitionIndex {
    /// Runs the existing semantic checker and freezes definition facts only on success.
    ///
    /// # Errors
    ///
    /// Returns the existing semantic diagnostics unchanged when the source is not a valid
    /// protocol-v2 scalar program.
    pub fn analyze(input: SemanticInput<'_>) -> Result<Self, Vec<Diagnostic>> {
        let syntax = input.syntax();
        let source_identity = input.sources().identity();
        let _ = lower(input)?;
        let mut records = Vec::new();
        for file in syntax.files() {
            for function in file.functions() {
                records.push(DefinitionRecord {
                    selection: function.name().span(),
                    declaration: function.name().span(),
                });
                for parameter in function.parameters() {
                    records.push(DefinitionRecord {
                        selection: parameter.name().span(),
                        declaration: parameter.name().span(),
                    });
                }
                for expression in function.body().expressions() {
                    let ExpressionKind::Reference { name } = expression.kind() else {
                        continue;
                    };
                    let Some(declaration) = function
                        .parameters()
                        .iter()
                        .find(|parameter| parameter.name().text() == name.text())
                        .map(|parameter| parameter.name().span())
                    else {
                        return Err(vec![Diagnostic::error_at(
                            "ZRYNA-M1006",
                            name.span(),
                            format!("name '{}' is not declared in this function", name.text()),
                            "reference one of the function's explicitly typed parameters",
                        )]);
                    };
                    records.push(DefinitionRecord { selection: name.span(), declaration });
                }
            }
        }
        Ok(Self { source_identity, records })
    }

    /// Returns whether these facts were issued from this exact source-map authority.
    #[must_use]
    pub fn is_bound_to(&self, sources: &SourceMap) -> bool {
        self.source_identity == sources.identity()
    }

    /// Returns the exact logical cache charge for retained span records.
    #[must_use]
    pub fn cache_bytes(&self) -> Option<usize> {
        self.records.len().checked_mul(DEFINITION_RECORD_CACHE_BYTES)
    }

    /// Resolves a byte inside a supported identifier, charging one authority comparison plus one
    /// unit for each examined record. Token ends, whitespace, comments, and EOF are absent.
    #[must_use]
    pub fn definition(&self, file: FileId, byte_offset: u32, work_limit: u64) -> DefinitionLookup {
        let mut work = 1_u64;
        if work > work_limit {
            return DefinitionLookup::OverBudget;
        }
        for record in &self.records {
            work = match work.checked_add(1) {
                Some(work) if work <= work_limit => work,
                _ => return DefinitionLookup::OverBudget,
            };
            if record.selection.file() == file
                && record.selection.start() <= byte_offset
                && byte_offset < record.selection.end()
            {
                return DefinitionLookup::Found(record.declaration);
            }
        }
        DefinitionLookup::Absent
    }
}
