//! Canonical operator-selected inventory; paths never originate from a submission.

use super::ERROR;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    format: String,
    version: u32,
    role: String,
    pub(super) directories: Vec<String>,
    pub(super) files: Vec<Material>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Material {
    pub(super) path: String,
    pub(super) bytes: u64,
    pub(super) sha256: String,
    pub(super) mode: u32,
}

fn portable(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && path.split('/').count() <= 16
        && path.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._/@-".contains(&byte))
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

pub(super) fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

pub(super) fn name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or("")
}

pub(super) fn parse(bytes: &[u8]) -> Result<Manifest, &'static str> {
    if bytes.is_empty() || bytes.len() > 262_144 {
        return Err(ERROR);
    }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| ERROR)?;
    if serde_json::to_vec(&manifest).map_err(|_| ERROR)? != bytes
        || manifest.format != "zryna.playground-materials.v1"
        || manifest.version != 1
        || manifest.directories.len() > 512
        || manifest.files.is_empty()
    {
        return Err(ERROR);
    }
    let (count, total, file_max) = match manifest.role.as_str() {
        "compiler" => (64, 192 * 1024 * 1024_u64, 128 * 1024 * 1024_u64),
        "evaluator" => (512, 1024 * 1024 * 1024_u64, 512 * 1024 * 1024_u64),
        _ => return Err(ERROR),
    };
    if manifest.files.len() > count {
        return Err(ERROR);
    }
    let mut directories = BTreeSet::from([""]);
    let mut previous = None;
    for path in &manifest.directories {
        if !portable(path)
            || previous.is_some_and(|old: &str| old >= path.as_str())
            || !directories.contains(parent(path))
            || !directories.insert(path.as_str())
        {
            return Err(ERROR);
        }
        previous = Some(path);
    }
    let mut previous = None;
    let mut size = 0_u64;
    for file in &manifest.files {
        size = size.checked_add(file.bytes).ok_or(ERROR)?;
        if !portable(&file.path)
            || previous.is_some_and(|old: &str| old >= file.path.as_str())
            || directories.contains(file.path.as_str())
            || !directories.contains(parent(&file.path))
            || file.bytes == 0
            || file.bytes > file_max
            || size > total
            || !matches!(file.mode, 0o444 | 0o555)
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ERROR);
        }
        previous = Some(&file.path);
    }
    Ok(manifest)
}
