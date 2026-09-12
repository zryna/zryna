#[cfg(not(windows))]
use std::{collections::BTreeSet, fs, path::Path};

#[cfg(not(windows))]
use same_file::Handle;
use sha2::{Digest as _, Sha256};

#[cfg(not(windows))]
use super::PackageBuildError;

#[cfg(windows)]
pub(super) fn audit_inventory_directory(
    root: &cap_std::fs::Dir,
    outputs: &[super::CacheOutput],
) -> Result<(), super::PackageBuildError> {
    if super::super::staging::inventory(root, super::PackageBuildError::cache)?
        != super::expected_inventory(outputs)
    {
        return Err(super::PackageBuildError::cache(
            "cache entry inventory contains missing or extra paths",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn collect_inventory(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeSet<String>,
) -> Result<(), PackageBuildError> {
    for item in fs::read_dir(directory)
        .map_err(|_| PackageBuildError::cache("cache entry cannot be enumerated"))?
    {
        let item =
            item.map_err(|_| PackageBuildError::cache("cache entry cannot be enumerated"))?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| PackageBuildError::cache("cache path cannot be inspected"))?;
        if link_like(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(PackageBuildError::cache("cache entry contains an unsafe path"));
        }
        #[cfg(unix)]
        if metadata.is_file() {
            use std::os::unix::fs::MetadataExt as _;
            if metadata.nlink() != 1 {
                return Err(PackageBuildError::cache("cache entry contains a hard-link alias"));
            }
        }
        let relative =
            path.strip_prefix(root).map_err(|_| PackageBuildError::cache("cache path escaped"))?;
        entries.insert(relative.to_string_lossy().replace('\\', "/"));
        if metadata.is_dir() {
            collect_inventory(root, &path, entries)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn read_stable_file(path: &Path, limit: usize) -> Result<Vec<u8>, PackageBuildError> {
    let before = Handle::from_path(path)
        .map_err(|_| PackageBuildError::cache("cache file cannot be retained"))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| PackageBuildError::cache("cache file cannot be inspected"))?;
    if !metadata.is_file()
        || link_like(&metadata)
        || metadata.len() > u64::try_from(limit).unwrap_or(u64::MAX)
    {
        return Err(PackageBuildError::cache("cache file is unsafe or exceeds its limit"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.nlink() != 1 {
            return Err(PackageBuildError::cache("cache file has a hard-link alias"));
        }
    }
    let bytes =
        fs::read(path).map_err(|_| PackageBuildError::cache("cache file cannot be read"))?;
    let after = Handle::from_path(path)
        .map_err(|_| PackageBuildError::cache("cache file cannot be revalidated"))?;
    if before != after || bytes.len() > limit {
        return Err(PackageBuildError::cache("cache file changed during validation"));
    }
    Ok(bytes)
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(not(windows))]
pub(super) fn link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
