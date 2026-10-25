//! Typed source identities and explicitly opted-in immutable compiler fragments only.
use super::super::{Writer, internal};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{invocation::SourceStatus, raw::TrapIdentity};
const IDENTITIES: [(TrapIdentity, &str); 5] = [
    (TrapIdentity::Bounds, "zryna.trap.bounds-v1"),
    (TrapIdentity::Allocation, "zryna.trap.allocation-v1"),
    (TrapIdentity::Capacity, "zryna.trap.capacity-v1"),
    (TrapIdentity::Refcount, "zryna.trap.refcount-v1"),
    (TrapIdentity::Utf8, "zryna.trap.utf8-v1"),
];
pub(in crate::generic_execution_v1) fn identity(value: &str) -> Option<TrapIdentity> {
    IDENTITIES.iter().find_map(|(identity, text)| (*text == value).then_some(*identity))
}
pub(in crate::generic_execution_v1) fn emit(
    w: &mut Writer,
    identity: TrapIdentity,
    invocation: bool,
) -> Result<(), Diagnostic> {
    if invocation {
        w.format(format_args!("throw $sourceTraps[{}];\n", SourceStatus::Trap(identity).code()))
    } else {
        let text = IDENTITIES
            .iter()
            .find_map(|(key, text)| (*key == identity).then_some(*text))
            .ok_or_else(internal)?;
        w.format(format_args!("throw new Error('{text}');\n"))
    }
}
// Prefix and closing delimiter scans advance across disjoint compiler-owned segments.
// A matched identity is compared against only the five fixed typed cases.
// No rendered module or user-provided string/data enters this function.
pub(in crate::generic_execution_v1) fn fragment(
    w: &mut Writer,
    mut text: &'static str,
) -> Result<(), Diagnostic> {
    const PREFIX: &str = "throw new Error('zryna.trap.";
    while let Some(start) = text.find(PREFIX) {
        let suffix = &text[start + PREFIX.len()..];
        let end = suffix.find("')").ok_or_else(internal)?;
        let code = match &suffix[..end] {
            "bounds-v1" => 1,
            "allocation-v1" => 2,
            "capacity-v1" => 3,
            "refcount-v1" => 4,
            "utf8-v1" => 5,
            _ => return Err(internal()),
        };
        w.text(&text[..start])?;
        w.format(format_args!("throw $sourceTraps[{code}]"))?;
        text = &suffix[end + 2..];
    }
    w.text(text)
}
