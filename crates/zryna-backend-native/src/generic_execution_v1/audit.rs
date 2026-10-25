//! Closed ELF inventory with independently derived source-call/runtime multiplicities.
use super::{invariant, runtime};
use object::Object;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod address;
mod container_edges;
mod edges;
pub(super) mod indexed;
mod inventory;
pub(super) mod push_request;
mod relocations;
mod sections;
pub(super) fn check(bytes: &[u8], p: &VerifiedProgram<'_, '_>) -> Result<(), Diagnostic> {
    check_entry(bytes, p, super::invocation::Entry::Ordinary)
}
pub(super) fn check_invocation(
    bytes: &[u8],
    p: &VerifiedProgram<'_, '_>,
    plan: &super::invocation::plan::Plan<'_>,
) -> Result<(), Diagnostic> {
    check_entry(bytes, p, super::invocation::Entry::from_plan(Some(plan)))
}
pub(super) fn check_entry(
    bytes: &[u8],
    p: &VerifiedProgram<'_, '_>,
    entry: super::invocation::Entry,
) -> Result<(), Diagnostic> {
    if bytes.len() > crate::MAX_NATIVE_OBJECT_BYTES {
        return Err(error());
    }
    let file = object::File::parse(bytes).map_err(|_| error())?;
    if file.format() != object::BinaryFormat::Elf
        || file.architecture() != object::Architecture::X86_64
        || file.endianness() != object::Endianness::Little
        || file.kind() != object::ObjectKind::Relocatable
        || !file.is_64()
        || file.dynamic_symbols().next().is_some()
        || file.dynamic_relocations().is_some_and(|mut r| r.next().is_some())
    {
        return Err(error());
    }
    sections::check(&file, p.requires_recursive_arena())?;
    let imports = runtime::names(p)?;
    let inventory = inventory::check(&file, p, &imports, entry)?;
    let expected_edges = edges::edges(p, &imports, entry)?;
    relocations::check(&file, &inventory, expected_edges, p.requires_recursive_arena())
}
pub(in crate::generic_execution_v1) fn error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N7103",
        None,
        "full execution ELF failed its exact runtime/call inventory audit",
        "retain the new source-bound execution program and exact runtime declarations",
    )
}
