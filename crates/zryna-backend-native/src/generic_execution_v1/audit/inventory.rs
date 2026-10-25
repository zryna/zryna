//! Exact ELF symbol inventory; retained names come from the independently matched program.
//! Object byte names are compared, never borrowed as program-owned edge keys.
use super::error;
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind, SymbolScope, SymbolSection};
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) struct Inventory<'program> {
    pub symbols: BTreeMap<usize, (&'program str, u64, u64)>,
    pub extents: Vec<(u64, u64, &'program str)>,
}
pub(super) fn check<'program>(
    file: &object::File<'_>,
    p: &'program VerifiedProgram<'_, '_>,
    imports: &[&'program str],
    entry: super::super::invocation::Entry,
) -> Result<Inventory<'program>, Diagnostic> {
    let text = file.section_by_name(".text");
    let expected = expected_symbols(p, entry);
    let mut tls_seen = false;
    let mut observed = BTreeSet::new();
    let mut observed_imports = BTreeSet::new();
    let mut symbols = BTreeMap::new();
    let mut extents = Vec::new();
    let mut files = 0;
    for symbol in file.symbols() {
        let name = symbol.name().map_err(|_| error())?;
        if symbol.kind() == SymbolKind::File {
            file_symbol(&symbol, &mut files)?;
            continue;
        }
        if symbol.kind() == SymbolKind::Tls || name == super::super::recursive::TLS_NAME {
            if tls_seen {
                return Err(error());
            }
            super::super::recursive::audit::symbol(file, &symbol, p.requires_recursive_arena())?;
            tls_seen = true;
            continue;
        }
        let canonical = if symbol.is_undefined() {
            if !imports.contains(&name)
                || symbol.kind() != SymbolKind::Unknown
                || symbol.scope() != SymbolScope::Unknown
                || symbol.flags() != (object::SymbolFlags::Elf { st_info: 16, st_other: 0 })
                || !symbol.is_global()
                || symbol.is_weak()
                || symbol.address() != 0
                || symbol.size() != 0
                || !observed_imports.insert(name)
            {
                return Err(error());
            }
            imports.iter().copied().find(|candidate| *candidate == name).ok_or_else(error)?
        } else {
            let (expected_name, expected_scope) = expected.get_key_value(name).ok_or_else(error)?;
            if *expected_scope != symbol.scope()
                || symbol.kind() != SymbolKind::Text
                || symbol.is_weak()
                || symbol.section()
                    != text
                        .as_ref()
                        .map_or(SymbolSection::None, |s| SymbolSection::Section(s.index()))
                || symbol.size() == 0
                || symbol.flags()
                    != (object::SymbolFlags::Elf {
                        st_info: if symbol.scope() == SymbolScope::Compilation { 2 } else { 18 },
                        st_other: 0,
                    })
                || !observed.insert(name)
            {
                return Err(error());
            }
            let end = symbol
                .address()
                .checked_add(symbol.size())
                .filter(|v| *v <= text.as_ref().map_or(0, ObjectSection::size))
                .ok_or_else(error)?;
            let canonical = *expected_name;
            extents.push((symbol.address(), end, canonical));
            canonical
        };
        if symbols.insert(symbol.index().0, (canonical, symbol.address(), symbol.size())).is_some()
        {
            return Err(error());
        }
    }
    if tls_seen != p.requires_recursive_arena()
        || files != 1
        || observed != expected.keys().copied().collect()
        || observed_imports != imports.iter().copied().collect()
    {
        return Err(error());
    }
    if p.requires_recursive_arena() {
        super::super::recursive::audit::stub(file)?;
    }
    extents.sort_unstable();
    if extents.windows(2).any(|v| v[0].1 > v[1].0) {
        return Err(error());
    }
    Ok(Inventory { symbols, extents })
}

fn expected_symbols<'program>(
    p: &'program VerifiedProgram<'_, '_>,
    entry: super::super::invocation::Entry,
) -> BTreeMap<&'program str, SymbolScope> {
    let mut expected = BTreeMap::new();
    for f in p.functions() {
        expected.insert(f.symbol(), SymbolScope::Compilation);
    }
    if let Some(plan) = p.containers() {
        for name in plan.clone_symbols().chain(plan.drop_symbols()).flatten() {
            expected.insert(name, SymbolScope::Compilation);
        }
    }
    for (name, _) in entry.wrappers(p) {
        expected.insert(name, SymbolScope::Dynamic);
    }
    if p.requires_recursive_arena() {
        expected.insert(super::super::recursive::STUB_NAME, SymbolScope::Compilation);
        for kind in super::super::recursive::calls::KINDS {
            expected.insert(super::super::recursive::calls::name(kind), SymbolScope::Compilation);
        }
    }
    expected
}
fn file_symbol(symbol: &object::Symbol<'_, '_>, files: &mut usize) -> Result<(), Diagnostic> {
    if symbol.name().map_err(|_| error())? != "zryna-gexecution-v1"
        || symbol.scope() != SymbolScope::Compilation
        || symbol.address() != 0
        || symbol.size() != 0
        || symbol.section() != SymbolSection::None
        || symbol.flags() != (object::SymbolFlags::Elf { st_info: 4, st_other: 0 })
        || *files != 0
    {
        return Err(error());
    }
    *files += 1;
    Ok(())
}
