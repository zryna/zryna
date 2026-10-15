//! Bounded readonly native sysroot snapshots, with no filesystem authority.
use super::identity::{canonical, sha};
use super::{NativeCompileStep, NativeRecipeError, denied};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}
fn path(value: &str) -> bool {
    zryna_package::validate_source(&zryna_package::PackageSource {
        kind: zryna_package::PackageSourceKind::Local,
        locator: value.to_owned(),
        revision: String::new(),
    })
    .is_ok()
}
pub(super) fn sysroots(step: &NativeCompileStep) -> Result<Vec<Value>, NativeRecipeError> {
    if step.sysroots.len() > 8 || step.sysroots.windows(2).any(|pair| pair[0].id >= pair[1].id) {
        return Err(denied("TRUST-NATIVE", "sysroots must be bounded sorted unique identities"));
    }
    let (mut files, mut bytes) = (0usize, 0usize);
    let mut inventory = Vec::new();
    for material in &step.sysroots {
        if !name(&material.id) || material.files.is_empty() {
            return Err(denied("TRUST-NATIVE", "sysroot identity or inventory is invalid"));
        }
        let paths: BTreeSet<_> = material.files.keys().map(String::as_str).collect();
        let mut entries = Vec::new();
        for (file, body) in &material.files {
            files += 1;
            bytes = bytes
                .checked_add(body.len())
                .ok_or_else(|| denied("TRUST-NATIVE", "readonly material bound exceeded"))?;
            if files > 64
                || bytes > 32 * 1024 * 1024
                || !path(file)
                || file
                    .split('/')
                    .scan(String::new(), |prefix, part| {
                        if !prefix.is_empty() {
                            prefix.push('/');
                        }
                        prefix.push_str(part);
                        Some(prefix.clone())
                    })
                    .any(|prefix| prefix != *file && paths.contains(prefix.as_str()))
            {
                return Err(denied(
                    "TRUST-NATIVE",
                    "readonly material inventory is outside its bounds",
                ));
            }
            entries.push(json!({"path":file,"size":body.len(),"sha256":sha(body)}));
        }
        let digest = sha(&canonical(&json!(entries))?);
        inventory.push(json!({"id":material.id,"target":"native-linux-x86_64","sha256":digest,"files":entries}));
    }
    Ok(inventory)
}
