//! Exact direct-call relocation encodings, owning extents and independent edge multiplicity.
use super::{error, inventory::Inventory};
use object::{
    Object, ObjectSection, RelocationEncoding, RelocationFlags, RelocationKind, RelocationTarget,
};
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
pub(super) fn check<'program>(
    file: &object::File<'_>,
    inventory: &Inventory<'program>,
    mut expected_edges: BTreeMap<(&'program str, &'program str), usize>,
) -> Result<(), Diagnostic> {
    let text = file.section_by_name(".text");
    let Inventory { symbols, extents } = inventory;
    let mut previous = None;
    if let Some(text) = text {
        let code = text.data().map_err(|_| error())?;
        for (offset, r) in text.relocations() {
            let caller = extents
                .iter()
                .find(|(begin, end, _)| {
                    offset > *begin && offset.checked_add(4).is_some_and(|v| v <= *end)
                })
                .map(|v| v.2)
                .ok_or_else(error)?;
            let RelocationTarget::Symbol(target) = r.target() else {
                return Err(error());
            };
            let target = symbols.get(&target.0).ok_or_else(error)?.0;
            if code.get(
                usize::try_from(offset.checked_sub(1).ok_or_else(error)?).map_err(|_| error())?,
            ) != Some(&0xe8)
                || previous.is_some_and(|v| v >= offset)
                || r.kind() != RelocationKind::PltRelative
                || r.encoding() != RelocationEncoding::X86Branch
                || r.size() != 32
                || r.addend() != -4
                || r.has_implicit_addend()
                || r.subtractor().is_some()
                || r.flags() != (RelocationFlags::Elf { r_type: object::elf::R_X86_64_PLT32 })
            {
                return Err(error());
            }
            let count = expected_edges.get_mut(&(caller, target)).ok_or_else(error)?;
            *count = count.checked_sub(1).ok_or_else(error)?;
            previous = Some(offset);
        }
    }
    if expected_edges.values().any(|n| *n != 0) {
        return Err(error());
    }
    Ok(())
}
