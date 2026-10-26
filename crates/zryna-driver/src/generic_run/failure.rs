//! Preserve cleanup failures across vector-valued native preparation/execution diagnostics.
use crate::{CommandFailure, CommandFailureKind};
use zryna_diagnostics::Diagnostic;
pub(super) fn boundary(message: &'static str) -> CommandFailure {
    preparation(Diagnostic::error(
        "ZRYNA-C3413",
        None,
        message,
        "retain the same source receipt, selected target and captured host capabilities",
    ))
}
pub(super) fn preparation(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Preparation, diagnostics: vec![diagnostic] }
}
pub(super) fn native(diagnostics: Vec<Diagnostic>, phase: CommandFailureKind) -> CommandFailure {
    let kind = if diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.code(), "ZRYNA-N4009" | "ZRYNA-N4016"))
    {
        CommandFailureKind::Cleanup
    } else {
        phase
    };
    CommandFailure { kind, diagnostics }
}
