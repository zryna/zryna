//! Preserve owning source diagnostics; infrastructure does not become a source resource limit.
use crate::{CommandFailure, CommandFailureKind};
use zryna_diagnostics::Diagnostic;
pub(super) fn source(diagnostics: Vec<Diagnostic>) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Source, diagnostics }
}
fn infrastructure(message: &'static str) -> CommandFailure {
    CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: vec![Diagnostic::error(
            "ZRYNA-C3401",
            None,
            message,
            "retain the exact generic source authority and report the smallest reproducible compiler failure",
        )],
    }
}
pub(super) fn invariant() -> CommandFailure {
    infrastructure("scoped generic source authority invariant failed")
}
fn allocation() -> CommandFailure {
    infrastructure("scoped generic compiler temporary allocation failed")
}
pub(super) fn body(
    error: zryna_semantics::bounded_generics_v1::body_types::BodyTypeFailure,
) -> CommandFailure {
    use zryna_semantics::bounded_generics_v1::body_types::BodyTypeFailure as F;
    match error {
        F::Diagnostics(items) => source(items),
        F::AllocationFailure => allocation(),
        F::InternalFailure => invariant(),
    }
}
pub(super) fn execution(error: zryna_ir::generic_v1::Failure) -> CommandFailure {
    use zryna_ir::generic_v1::Failure as F;
    match error {
        F::Diagnostics(items) => source(items),
        F::AllocationFailure => allocation(),
        F::InternalFailure => invariant(),
    }
}
pub(super) fn layout(error: zryna_layout::generic_v1::Failure) -> CommandFailure {
    use zryna_layout::generic_v1::Failure as F;
    match error {
        F::Diagnostics(items) => source(items),
        F::AllocationFailure => allocation(),
        F::InternalFailure => invariant(),
    }
}
pub(super) fn runtime(error: zryna_ownership_runtime_abi::generic_v1::Failure) -> CommandFailure {
    use zryna_ownership_runtime_abi::generic_v1::Failure as F;
    match error {
        F::AllocationFailure => allocation(),
        F::Violations(items) => source(
            items
                .into_iter()
                .map(|item| {
                    Diagnostic::error(
                        item.code(),
                        None,
                        item.message(),
                        "retain independently matched generic runtime declarations and layouts",
                    )
                })
                .collect(),
        ),
    }
}
pub(super) fn reserve<T>(count: usize) -> Result<Vec<T>, CommandFailure> {
    let mut items = Vec::new();
    items.try_reserve_exact(count).map_err(|_| allocation())?;
    Ok(items)
}
