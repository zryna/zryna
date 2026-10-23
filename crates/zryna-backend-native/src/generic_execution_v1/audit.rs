//! Closed ELF inventory with independently derived source-call/runtime multiplicities.
use super::{invariant, runtime};
use object::Object;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod container_edges;
mod edges;
mod inventory;
mod relocations;
mod sections;
pub(super) fn check(bytes: &[u8], p: &VerifiedProgram<'_, '_>) -> Result<(), Diagnostic> {
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
    sections::check(&file)?;
    let imports = runtime::names(p)?;
    let inventory = inventory::check(&file, p, &imports)?;
    let expected_edges = edges::edges(p, &imports)?;
    relocations::check(&file, &inventory, expected_edges)
}
fn error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-N7103",
        None,
        "full execution ELF failed its exact runtime/call inventory audit",
        "retain the new source-bound execution program and exact runtime declarations",
    )
}
