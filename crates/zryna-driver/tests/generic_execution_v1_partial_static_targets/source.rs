//! Reuse the exact two authenticated source owners, never a constructed proof or wrapper literal.
use std::path::{Path, PathBuf};
use zryna_ir::generic_v1::execution_v1;
#[path = "../generic_execution_v1_partial_canonical_empty/source.rs"]
mod empty;
#[path = "../generic_execution_v1_partial_static_source_seal/source.rs"]
mod partial;
pub(super) const PROVIDERS: [&str; 2] = partial::PROVIDERS;
pub(super) const CASES: [&str; 12] = [
    "struct-survivor",
    "struct-refill-whole",
    "partial-root-replacement",
    "nested-disjoint",
    "array-survivor",
    "array-refill",
    "opaque-child",
    "sibling-loan",
    "failure-after-hole",
    "branch-same-hole",
    "loop-refill",
    "whole-vs-fields",
];
pub(super) fn row(case: &str) -> serde_json::Value {
    if case == "whole-vs-fields" { empty::row(case) } else { partial::row(case) }
}
pub(super) fn root(case: &str) -> PathBuf {
    let name = if case == "whole-vs-fields" {
        "m7-generic-execution-partial-canonical-empty"
    } else {
        "m7-generic-execution-partial-static"
    };
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests").join(name)
}
pub(super) fn project(
    case: &str,
    provider: &str,
) -> (zryna_source::SourceMap, zryna_syntax::v5::VerifiedProjectSyntaxV5) {
    if case == "whole-vs-fields" {
        empty::project(case, provider)
    } else {
        partial::project(case, provider)
    }
}
pub(super) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    if case == "whole-vs-fields" {
        empty::with_program(case, provider, check);
    } else {
        partial::with_program(case, provider, check);
    }
}
