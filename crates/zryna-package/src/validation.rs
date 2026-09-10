use std::collections::BTreeSet;

use crate::{
    canonical::{digest, sha256},
    model::{Checksum, Compatibility, Manifest, PackageSource, PackageSourceKind},
    resolver::{PackageFile, ResolveError},
};

pub(crate) fn manifest(manifest: &Manifest) -> Result<(), ResolveError> {
    if manifest.format != "zryna.package.v1" {
        return Err(ResolveError::schema("unsupported package manifest format"));
    }
    name(&manifest.name)?;
    version(&manifest.version)?;
    source(&manifest.source)?;
    compatibility(&manifest.compatibility)?;
    if manifest.files.is_empty() || manifest.files.len() > 16 {
        return Err(ResolveError::budget("source file count is outside 1..=16"));
    }
    if manifest.dependencies.len() > 8 {
        return Err(ResolveError::budget("dependency count exceeds 8"));
    }
    ordered(&manifest.files, |item| &item.path, "source files")?;
    ordered(&manifest.dependencies, |item| &item.alias, "dependencies")?;
    for file in &manifest.files {
        portable_path(&file.path)?;
        if file.size > 1_024 || !is_digest(&file.sha256) {
            return Err(ResolveError::schema("invalid source checksum record"));
        }
    }
    reject_prefix_collisions(&manifest.files)?;
    for dependency in &manifest.dependencies {
        name(&dependency.alias)?;
        name(&dependency.name)?;
        version(&dependency.version)?;
        source(&dependency.source)?;
    }
    Ok(())
}

pub(crate) fn compatibility(value: &Compatibility) -> Result<(), ResolveError> {
    version(&value.compiler)?;
    if !matches!(value.profile.as_str(), "i32-v1" | "control-flow-v1" | "data-ownership-v1") {
        return Err(ResolveError::schema("unsupported language profile"));
    }
    if value.targets.is_empty() || value.targets.len() > 3 {
        return Err(ResolveError::budget("target count is outside 1..=3"));
    }
    ordered(&value.targets, String::as_str, "targets")?;
    if value.targets.iter().any(|target| {
        !matches!(target.as_str(), "javascript" | "native-linux-x86_64" | "webassembly")
    }) {
        return Err(ResolveError::schema("unsupported package target"));
    }
    Ok(())
}

pub(crate) fn source(value: &PackageSource) -> Result<(), ResolveError> {
    match value.kind {
        PackageSourceKind::Local => {
            if !value.revision.is_empty() {
                return Err(ResolveError::source("local source revision must be empty"));
            }
            portable_path(&value.locator)?;
        }
        PackageSourceKind::Git => {
            if !canonical_git_locator(&value.locator) || !is_commit(&value.revision) {
                return Err(ResolveError::source(
                    "Git source requires a canonical HTTPS .git locator and exact commit",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn authenticate_files(
    manifest: &Manifest,
    mut files: Vec<PackageFile>,
) -> Result<String, ResolveError> {
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files.len() != manifest.files.len() {
        return Err(ResolveError::source("source inventory is missing or contains extra files"));
    }
    for (expected, actual) in manifest.files.iter().zip(&files) {
        if expected.path != actual.path
            || usize::try_from(expected.size).ok() != Some(actual.bytes.len())
            || expected.sha256 != sha256(&actual.bytes)
        {
            return Err(ResolveError::source("source inventory bytes differ from the manifest"));
        }
    }
    digest("source-files", &manifest.files)
}

pub(crate) fn portable_path(value: &str) -> Result<(), ResolveError> {
    if value.is_empty()
        || value.len() > 96
        || !value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'/' | b'.' | b'_' | b'-'))
        })
    {
        return Err(ResolveError::path("invalid portable path"));
    }
    for segment in value.split('/') {
        let stem = segment.split('.').next().unwrap_or_default();
        let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
            || stem
                .strip_prefix("com")
                .is_some_and(|suffix| suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit())
            || stem
                .strip_prefix("lpt")
                .is_some_and(|suffix| suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit());
        if segment.is_empty() || matches!(segment, "." | "..") || segment.ends_with('.') || reserved
        {
            return Err(ResolveError::path("invalid portable path segment"));
        }
    }
    Ok(())
}

pub(crate) fn name(value: &str) -> Result<(), ResolveError> {
    if value.is_empty()
        || value.len() > 64
        || !value.as_bytes()[0].is_ascii_lowercase()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(ResolveError::schema("invalid package name or alias"));
    }
    Ok(())
}

fn version(value: &str) -> Result<(), ResolveError> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || value.len() > 14
        || parts.iter().any(|part| {
            part.is_empty()
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || part.parse::<u16>().map_or(true, |number| number > 9_999)
        })
    {
        return Err(ResolveError::schema("invalid exact package version"));
    }
    Ok(())
}

fn canonical_git_locator(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(".git") else {
        return false;
    };
    if value.len() > 96
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
        || value.contains(['?', '#', '@', '%', '\\'])
    {
        return false;
    }
    let Some((host, path)) = rest.split_once('/') else {
        return false;
    };
    !host.is_empty()
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && !host
            .as_bytes()
            .windows(2)
            .any(|pair| matches!(pair[0], b'.' | b'-') && matches!(pair[1], b'.' | b'-'))
        && host.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && !path.is_empty()
        && path.split('/').all(valid_git_segment)
}

fn valid_git_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn is_commit(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn ordered<T, F>(items: &[T], key: F, label: &'static str) -> Result<(), ResolveError>
where
    F: Fn(&T) -> &str,
{
    if items.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(ResolveError::order(label));
    }
    Ok(())
}

fn reject_prefix_collisions(files: &[Checksum]) -> Result<(), ResolveError> {
    let paths: BTreeSet<_> = files.iter().map(|file| file.path.as_str()).collect();
    for path in &paths {
        let mut offset = 0;
        while let Some(relative) = path[offset..].find('/') {
            offset += relative;
            if paths.contains(&path[..offset]) {
                return Err(ResolveError::path("file and directory paths collide"));
            }
            offset += 1;
        }
    }
    Ok(())
}
