use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

pub(super) fn validate(expected: &Expectation) -> Result<(), Diagnostic> {
    if expected.format != "zryna.private-link-inventory.v1"
        || expected.target != "x86_64-unknown-linux-gnu"
        || !expected.root.is_absolute()
        || expected.materials.is_empty()
        || expected.materials.len() > MAX_MATERIALS
        || expected.materials.iter().filter(|item| item.role == Role::Tool).count()
            > MAX_TOOL_MATERIALS
        || expected.linker_order.is_empty()
        || expected.linker_order.len() > 96
    {
        return Err(rejected());
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut prefixes = BTreeMap::new();
    let mut total = 0_usize;
    for item in &expected.materials {
        let components = item.path.split('/').collect::<Vec<_>>();
        if !name(&item.id)
            || !ids.insert(item.id.to_ascii_lowercase())
            || item.path.len() > 4096
            || components.iter().any(|part| !name(part))
            || !names.insert(item.path.to_ascii_lowercase())
            || item.bytes == 0
            || item.bytes
                > if item.role == Role::Tool {
                    MAX_TOOL_BYTES
                } else {
                    crate::native::MAX_NATIVE_EXECUTABLE_BYTES
                }
            || !canonical_original(&item.original_path)
            || (item.role == Role::Tool && item.trace_path.as_deref() != Some(&item.original_path))
            || item.trace_path.as_ref().is_some_and(|value| {
                value.is_empty() || value.len() > 4096 || value.contains(['\0', '\n', '\r'])
            })
        {
            return Err(rejected());
        }
        for index in 1..components.len() {
            let spelling = components[..index].join("/");
            if prefixes
                .insert(spelling.to_ascii_lowercase(), spelling.clone())
                .is_some_and(|previous| previous != spelling)
            {
                return Err(rejected());
            }
        }
        total = total.checked_add(item.bytes).ok_or_else(rejected)?;
    }
    if total > MAX_TOTAL_BYTES || prefixes.keys().any(|prefix| names.contains(prefix)) {
        return Err(rejected());
    }
    for provider in &expected.providers.providers {
        let item =
            expected.materials.iter().find(|item| item.id == provider.id).ok_or_else(rejected)?;
        if !matches!(item.role, Role::Shared | Role::Loader) || item.sha256 != provider.sha256 {
            return Err(rejected());
        }
    }
    for slot in &expected.linker_order {
        resolve(
            expected,
            slot,
            &[
                ("client", "client".into()),
                ("program", "program".into()),
                ("foreign", "foreign".into()),
                ("runtime", "runtime".into()),
            ],
        )?;
    }
    Ok(())
}

fn resolve(
    expected: &Expectation,
    slot: &str,
    objects: &[(&str, String)],
) -> Result<String, Diagnostic> {
    if let Some(id) = slot.strip_prefix("material:") {
        return expected
            .materials
            .iter()
            .find(|item| item.id == id)
            .and_then(|item| item.trace_path.clone())
            .ok_or_else(rejected);
    }
    if let Some(value) = slot.strip_prefix("member:") {
        let (id, member) = value.split_once(':').ok_or_else(rejected)?;
        let archive = expected
            .materials
            .iter()
            .find(|item| item.id == id && item.role == Role::Static)
            .and_then(|item| item.trace_path.as_ref())
            .ok_or_else(rejected)?;
        if member.is_empty() || member.len() > 256 || member.contains(['\0', '\n', '\r', '(', ')'])
        {
            return Err(rejected());
        }
        return Ok(format!("({archive}){member}"));
    }
    if !["client", "program", "foreign", "runtime"].contains(&slot) {
        return Err(rejected());
    }
    objects.iter().find(|(id, _)| *id == slot).map(|(_, path)| path.clone()).ok_or_else(rejected)
}

pub(super) fn check_order(
    expected: &Expectation,
    observed: &[String],
    objects: &[(&str, String)],
) -> Result<(), Diagnostic> {
    let resolved = expected
        .linker_order
        .iter()
        .map(|slot| resolve(expected, slot, objects))
        .collect::<Result<Vec<_>, _>>()?;
    if observed != resolved {
        return Err(rejected());
    }
    Ok(())
}

fn canonical_original(value: &str) -> bool {
    let path = std::path::Path::new(value);
    !value.is_empty()
        && value.len() <= 4096
        && !value.contains(['\0', '\n', '\r'])
        && path.is_absolute()
        && path.components().collect::<PathBuf>().as_os_str() == path.as_os_str()
}
