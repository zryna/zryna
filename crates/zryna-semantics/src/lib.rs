//! Permanent boundary for Zryna-owned semantic analysis and IR lowering.

#![forbid(unsafe_code)]

use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::Program;
use zryna_source::SourceMap;
use zryna_syntax::v2::ProjectSyntaxSnapshot;

/// Inputs that a future semantic implementation must consume without frontend authority.
#[derive(Clone, Copy, Debug)]
pub struct SemanticInput<'a> {
    syntax: &'a ProjectSyntaxSnapshot,
    sources: &'a SourceMap,
}

impl<'a> SemanticInput<'a> {
    /// Binds verified syntax to the exact authoritative source map used to construct it.
    ///
    /// Returns `None` when the snapshot was verified by a different source-map instance or
    /// contains a provider error that must stop compilation before semantic analysis.
    #[must_use]
    pub fn try_new(syntax: &'a ProjectSyntaxSnapshot, sources: &'a SourceMap) -> Option<Self> {
        (syntax.is_bound_to(sources)
            && syntax
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.severity() != Severity::Error))
        .then_some(Self { syntax, sources })
    }

    /// Returns the verified provider-neutral syntax project.
    #[must_use]
    pub const fn syntax(self) -> &'a ProjectSyntaxSnapshot {
        self.syntax
    }

    /// Returns the authoritative source map for semantic diagnostics.
    #[must_use]
    pub const fn sources(self) -> &'a SourceMap {
        self.sources
    }
}

/// Output contract for the future semantic lowering implementation.
pub type SemanticResult = Result<Program, Vec<Diagnostic>>;

#[cfg(test)]
mod tests {
    use super::SemanticInput;
    use zryna_source::{SourceFileInput, SourceMap};
    use zryna_syntax::v2::{decode_snapshot, verify_snapshot};

    fn sources() -> SourceMap {
        SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: "export function yes(): bool { return true; }".to_owned(),
        }])
        .expect("fixture source map must build")
    }

    #[test]
    fn semantic_input_rejects_a_different_source_map_instance() {
        let sources = sources();
        let raw = decode_snapshot(include_bytes!("../../../tests/fixtures/syntax-v2-valid.json"))
            .expect("checked-in protocol fixture must decode");
        let syntax = verify_snapshot(raw, &sources).expect("checked-in fixture must verify");

        assert!(SemanticInput::try_new(&syntax, &sources).is_some());
        assert!(SemanticInput::try_new(&syntax, &self::sources()).is_none());
    }

    #[test]
    fn semantic_input_rejects_provider_errors() {
        let sources = sources();
        let fixture = include_str!("../../../tests/fixtures/syntax-v2-valid.json");
        let with_error = fixture.replace(
            "\"diagnostics\": []",
            r#""diagnostics": [{"code":"ZRYNA-F2002","severity":"error","location":{"kind":"global"},"message":"unsupported syntax","guidance":"use supported syntax"}]"#,
        );
        let raw = decode_snapshot(with_error.as_bytes()).expect("error fixture must decode");
        let syntax = verify_snapshot(raw, &sources).expect("provider error must remain verifiable");

        assert!(SemanticInput::try_new(&syntax, &sources).is_none());
    }
}
