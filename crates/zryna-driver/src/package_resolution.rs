//! Retained filesystem capabilities for source-only package resolution.

mod filesystem;

use std::path::{Path, PathBuf};

use cap_fs_ext::DirExt as _;
use cap_std::fs::Dir;
use sha2::{Digest as _, Sha256};
use zryna_package::{
    LockMode, PackageFile, PackageMaterial, PackageSource, PackageSourceKind,
    PackageSourceProvider, ResolveError, ResolvedGraph,
};

use filesystem::{CapturedRoot, RetainedFile, ensure_same_directory, publish_lock, read_retained};

const MANIFEST_NAME: &str = "zryna.package.json";
const LOCK_NAME: &str = "zryna.lock.json";
const MAX_MANIFEST_BYTES: usize = 65_536;
const MAX_SOURCE_BYTES: usize = 1_024;
const MAX_SOURCE_FILES: usize = 16;
const MAX_DIRECTORY_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Lock behavior for a driver-owned package resolution request.
pub enum PackageLockMode {
    /// Verify an existing exact lock without mutation.
    Frozen,
    /// Atomically publish the newly authenticated lock.
    Update,
}

#[derive(Clone, Debug)]
/// Driver request for one source-only package graph.
pub struct PackageResolutionRequest {
    /// Absolute declared reproduction/source root.
    pub source_root: PathBuf,
    /// Portable root-package locator relative to `source_root`.
    pub package: String,
    /// Optional absolute prepopulated exact-commit Git material cache.
    pub git_cache: Option<PathBuf>,
    /// Frozen verification or update publication behavior.
    pub mode: PackageLockMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Complete authenticated graph and lock publication observation.
pub struct PackageResolutionSuccess {
    graph: ResolvedGraph,
    lock_path: PathBuf,
    published: bool,
}

impl PackageResolutionSuccess {
    #[must_use]
    /// Returns the authenticated graph.
    pub fn graph(&self) -> &ResolvedGraph {
        &self.graph
    }

    #[must_use]
    /// Returns the root package lock path.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    #[must_use]
    /// Reports whether update mode atomically published the lock.
    pub fn published(&self) -> bool {
        self.published
    }
}

/// Resolves one package graph through retained local and optional Git-cache capabilities.
///
/// # Errors
///
/// Returns a stable package rejection without publishing a partial or unauthenticated lock.
pub fn resolve_package(
    request: &PackageResolutionRequest,
) -> Result<PackageResolutionSuccess, ResolveError> {
    let source_root = CapturedRoot::capture(&request.source_root)?;
    let git_cache = request.git_cache.as_deref().map(CapturedRoot::capture).transpose()?;
    let root_source = PackageSource {
        kind: PackageSourceKind::Local,
        locator: request.package.clone(),
        revision: String::new(),
    };
    zryna_package::validate_source(&root_source)?;
    let mut provider = FilesystemProvider { source_root, git_cache, retained: Vec::new() };
    let package_dir = provider.open_source_dir(&root_source)?;
    let existing_lock = match request.mode {
        PackageLockMode::Frozen => {
            let (bytes, retained) = read_retained(&package_dir, LOCK_NAME, MAX_MANIFEST_BYTES)?;
            provider.push_retained(retained)?;
            Some(bytes)
        }
        PackageLockMode::Update => None,
    };
    let mode = existing_lock.as_deref().map_or(LockMode::Update, LockMode::Frozen);
    let graph = zryna_package::resolve(&mut provider, root_source, mode)?;
    provider.revalidate()?;
    let current_package_dir = provider.open_source_dir(&PackageSource {
        kind: PackageSourceKind::Local,
        locator: request.package.clone(),
        revision: String::new(),
    })?;
    ensure_same_directory(&package_dir, &current_package_dir)?;
    let published = request.mode == PackageLockMode::Update;
    if published {
        publish_lock(&package_dir, LOCK_NAME, graph.lock_bytes())?;
    }
    Ok(PackageResolutionSuccess {
        graph,
        lock_path: request.source_root.join(&request.package).join(LOCK_NAME),
        published,
    })
}

struct FilesystemProvider {
    source_root: CapturedRoot,
    git_cache: Option<CapturedRoot>,
    retained: Vec<RetainedFile>,
}

impl PackageSourceProvider for FilesystemProvider {
    fn load(&mut self, source: &PackageSource) -> Result<PackageMaterial, ResolveError> {
        let directory = self.open_source_dir(source)?;
        let (manifest, retained) = read_retained(&directory, MANIFEST_NAME, MAX_MANIFEST_BYTES)?;
        self.push_retained(retained)?;
        let mut files = Vec::new();
        self.collect_files(&directory, "", 0, &mut files)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(PackageMaterial { manifest, files })
    }
}

impl FilesystemProvider {
    fn open_source_dir(&self, source: &PackageSource) -> Result<Dir, ResolveError> {
        match source.kind {
            PackageSourceKind::Local => self.source_root.open_relative(&source.locator),
            PackageSourceKind::Git => {
                let cache = self.git_cache.as_ref().ok_or_else(|| {
                    ResolveError::source("exact Git source is absent from the configured cache")
                })?;
                cache.open_relative(&git_cache_key(source))
            }
        }
    }

    fn collect_files(
        &mut self,
        directory: &Dir,
        prefix: &str,
        depth: usize,
        files: &mut Vec<PackageFile>,
    ) -> Result<(), ResolveError> {
        if depth > MAX_DIRECTORY_DEPTH {
            return Err(ResolveError::source("package directory depth exceeds 32"));
        }
        let mut names = directory
            .entries()
            .map_err(|_| ResolveError::source("package directory cannot be enumerated"))?
            .map(|entry| {
                entry
                    .map_err(|_| ResolveError::source("package directory entry cannot be read"))
                    .and_then(|entry| {
                        entry.file_name().into_string().map_err(|_| {
                            ResolveError::source("package entry name is not portable UTF-8")
                        })
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        if names.windows(2).any(|pair| pair[0].eq_ignore_ascii_case(&pair[1])) {
            return Err(ResolveError::source("package entries collide under ASCII case folding"));
        }
        for name in names {
            if prefix.is_empty() && matches!(name.as_str(), MANIFEST_NAME | LOCK_NAME) {
                continue;
            }
            let metadata = directory
                .symlink_metadata(&name)
                .map_err(|_| ResolveError::source("package entry cannot be inspected"))?;
            if metadata.is_symlink() {
                return Err(ResolveError::source(
                    "package source contains a link or reparse point",
                ));
            }
            let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            if metadata.is_dir() {
                let child = directory
                    .open_dir_nofollow(&name)
                    .map_err(|_| ResolveError::source("package directory cannot be retained"))?;
                self.collect_files(&child, &path, depth + 1, files)?;
            } else if metadata.is_file() {
                if files.len() == MAX_SOURCE_FILES {
                    return Err(ResolveError::source("package source file count exceeds 16"));
                }
                let (bytes, retained) = read_retained(directory, &name, MAX_SOURCE_BYTES)?;
                self.push_retained(retained)?;
                files.push(PackageFile { path, bytes });
            } else {
                return Err(ResolveError::source("package source contains a special file"));
            }
        }
        Ok(())
    }

    fn revalidate(&mut self) -> Result<(), ResolveError> {
        for retained in &mut self.retained {
            retained.revalidate()?;
        }
        self.source_root.revalidate()?;
        if let Some(cache) = &self.git_cache {
            cache.revalidate()?;
        }
        Ok(())
    }

    fn push_retained(&mut self, retained: RetainedFile) -> Result<(), ResolveError> {
        if self.retained.iter().any(|existing| existing.handle == retained.handle) {
            return Err(ResolveError::source("package inputs contain a hard-link alias"));
        }
        self.retained.push(retained);
        Ok(())
    }
}

fn git_cache_key(source: &PackageSource) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ZRYNA-PACKAGE-GIT-CACHE-V1\0");
    hasher.update(source.locator.as_bytes());
    hasher.update(b"\0");
    hasher.update(source.revision.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "package_resolution_tests.rs"]
mod tests;
