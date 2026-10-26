//! Match the complete explicitly retained provider graph and versioned imports.
use super::{
    ClosureInventory, Diagnostic, Digest, ObjectInventory, ProviderExpectation, RetainedProvider,
    Sha256, dynamic, image, inspect, rejected, resolution,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn audit(
    final_elf: &[u8],
    materials: &[RetainedProvider],
    expected: &ProviderExpectation,
) -> Result<ClosureInventory, Diagnostic> {
    let executable = inspect(final_elf, 2, expected)?;
    if executable.interpreter.as_deref() != Some(&expected.interpreter)
        || executable.needed != expected.needed
        || executable.soname.is_some()
    {
        return Err(rejected());
    }
    let (result, names) = retain(
        materials,
        expected,
        final_elf.len(),
        executable.symbols.len(),
        executable.name_bytes(),
    )?;
    resolution::Index::new(&result, &names)?.check(&executable)?;
    Ok(ClosureInventory {
        executable,
        providers: result,
        execution_authorized: false,
        complete_input_use_attestation: false,
        relocation_or_resolver_execution_attested: false,
    })
}

type RetainedGraph = (Vec<(String, ObjectInventory)>, BTreeMap<String, usize>);
pub(super) fn retain(
    materials: &[RetainedProvider],
    expected: &ProviderExpectation,
    mut total: usize,
    mut symbol_count: usize,
    mut name_bytes: usize,
) -> Result<RetainedGraph, Diagnostic> {
    if materials.is_empty()
        || materials.len() > 32
        || materials.len() != expected.providers.len()
        || expected.needed.len() > 32
        || expected.interpreter.len() > 4096
        || !expected.interpreter.starts_with('/')
        || expected.interpreter.split('/').skip(1).any(|s| !dynamic::logical(s))
        || expected.needed.iter().any(|s| !dynamic::logical(s))
        || expected.needed.iter().collect::<BTreeSet<_>>().len() != expected.needed.len()
    {
        return Err(rejected());
    }
    let mut result = Vec::new();
    let mut names = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for (material, rule) in materials.iter().zip(&expected.providers) {
        total = total.checked_add(material.bytes.len()).ok_or_else(rejected)?;
        if total > 134_217_728
            || material.id != rule.id
            || !dynamic::logical(&rule.id)
            || rule.needed.len() > 32
            || !ids.insert(rule.id.clone())
            || !hashes.insert(rule.sha256)
            || <[u8; 32]>::from(Sha256::digest(&material.bytes)) != rule.sha256
        {
            return Err(rejected());
        }
        let provider = inspect(&material.bytes, 3, expected)?;
        let soname = rule.soname.as_ref().ok_or_else(rejected)?;
        if !dynamic::logical(soname) || names.insert(soname.clone(), result.len()).is_some() {
            return Err(rejected());
        }
        symbol_count = symbol_count.checked_add(provider.symbols.len()).ok_or_else(rejected)?;
        name_bytes = name_bytes.checked_add(provider.name_bytes()).ok_or_else(rejected)?;
        if symbol_count > image::MAX_SYMBOLS || name_bytes > image::MAX_STRINGS {
            return Err(rejected());
        }
        if provider.soname != rule.soname
            || provider.needed != rule.needed
            || provider.interpreter.as_ref().is_some_and(|name| name != &expected.interpreter)
        {
            return Err(rejected());
        }
        result.push((rule.id.clone(), provider));
    }
    let interpreter = expected.interpreter.rsplit('/').next().ok_or_else(rejected)?;
    if !names.contains_key(interpreter) {
        return Err(rejected());
    }
    let mut reachable = BTreeSet::new();
    let mut active = BTreeSet::new();
    let interpreter_name = interpreter.to_owned();
    for name in expected.needed.iter().chain(std::iter::once(&interpreter_name)) {
        visit(name, &names, &result, &mut reachable, &mut active)?;
    }
    if reachable.len() != materials.len() {
        return Err(rejected());
    }
    let index = resolution::Index::new(&result, &names)?;
    for (_, provider) in &result {
        index.check(provider)?;
    }
    Ok((result, names))
}

fn visit(
    name: &str,
    names: &BTreeMap<String, usize>,
    providers: &[(String, ObjectInventory)],
    reached: &mut BTreeSet<String>,
    active: &mut BTreeSet<String>,
) -> Result<(), Diagnostic> {
    if active.contains(name) {
        return Err(rejected());
    }
    if reached.contains(name) {
        return Ok(());
    }
    let index = *names.get(name).ok_or_else(rejected)?;
    active.insert(name.to_owned());
    for dependency in &providers[index].1.needed {
        visit(dependency, names, providers, reached, active)?;
    }
    active.remove(name);
    reached.insert(name.to_owned());
    Ok(())
}
