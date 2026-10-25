//! Authenticated original loan/concat bodies precede target emission and runtime observation.
mod concat_cases;
mod concat_negatives;
mod concat_source;
mod negatives;
mod quotas;
mod run;
mod runtime_cases;
mod source;
mod source_controls;

pub(super) fn catalogue(
    provider: &str,
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    source::with_program("referent-catalogue", provider, check);
}
