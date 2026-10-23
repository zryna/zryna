//! Closed section categories, flags, uniqueness and relocation locations.
use super::error;
use object::{Object, ObjectSection, SectionFlags, SectionKind};
use std::collections::BTreeSet;
use zryna_diagnostics::Diagnostic;
pub(super) fn check(file: &object::File<'_>) -> Result<(), Diagnostic> {
    let sections = file.sections().collect::<Vec<_>>();
    let mut section_names = BTreeSet::new();
    for s in &sections {
        let name = s.name().map_err(|_| error())?;
        let (kind, flags) = match name {
            ".text" => (SectionKind::Text, 6),
            ".rela.text" => (SectionKind::Metadata, 64),
            ".note.GNU-stack" => (SectionKind::Other, 0),
            ".symtab" | ".strtab" | ".shstrtab" => (SectionKind::Metadata, 0),
            _ => return Err(error()),
        };
        if s.kind() != kind
            || s.flags() != (SectionFlags::Elf { sh_flags: flags })
            || !section_names.insert(name)
            || (name != ".text" && s.relocations().next().is_some())
        {
            return Err(error());
        }
    }
    if ![".note.GNU-stack", ".symtab", ".strtab", ".shstrtab"]
        .iter()
        .all(|n| section_names.contains(n))
    {
        return Err(error());
    }
    Ok(())
}
