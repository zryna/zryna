use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    canonical::{digest, encode, parse},
    identity::PackageInstance,
    model::{Compatibility, Lock, LockEdge, LockPackage, Manifest, PackageSource},
    validation,
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// One provider-returned package-relative source file.
pub struct PackageFile {
    /// Portable package-relative source path.
    pub path: String,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Bounded manifest and source inventory returned through a caller-owned capability.
pub struct PackageMaterial {
    /// Exact canonical `zryna.package.v1` bytes.
    pub manifest: Vec<u8>,
    /// Complete declared package source inventory.
    pub files: Vec<PackageFile>,
}

/// Caller-owned source capability consumed by the pure resolver.
pub trait PackageSourceProvider {
    /// Returns the exact material selected by `source` without fallback.
    ///
    /// # Errors
    ///
    /// Returns a source error without material when input is absent, unsafe, or unstable.
    fn load(&mut self, source: &PackageSource) -> Result<PackageMaterial, ResolveError>;
}

#[derive(Clone, Copy, Debug)]
/// Lock handling selected for one resolution request.
pub enum LockMode<'a> {
    /// Resolve and return new canonical lock bytes.
    Update,
    /// Require these canonical lock bytes to exactly replay the resolved graph.
    Frozen(&'a [u8]),
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One authenticated package and its exact resolved edges.
pub struct ResolvedPackage {
    instance: PackageInstance,
    name: String,
    version: String,
    source: PackageSource,
    dependencies: Vec<(String, String)>,
}

impl ResolvedPackage {
    #[must_use]
    /// Returns the exact package instance.
    pub fn instance(&self) -> &PackageInstance {
        &self.instance
    }

    #[must_use]
    /// Returns the declared package name.
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Returns the declared exact package version.
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    /// Returns the authenticated source tuple.
    pub fn source(&self) -> &PackageSource {
        &self.source
    }

    #[must_use]
    /// Returns sorted `(alias, selected manifest id)` edges.
    pub fn dependencies(&self) -> &[(String, String)] {
        &self.dependencies
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Opaque authenticated source-only dependency graph and canonical lock.
pub struct ResolvedGraph {
    root: String,
    compatibility: Compatibility,
    packages: Vec<ResolvedPackage>,
    lock_bytes: Vec<u8>,
    lock_sha256: String,
}

impl ResolvedGraph {
    #[must_use]
    /// Returns the root manifest identity.
    pub fn root(&self) -> &str {
        &self.root
    }

    #[must_use]
    /// Returns exact graph compatibility.
    pub fn compatibility(&self) -> &Compatibility {
        &self.compatibility
    }

    #[must_use]
    /// Returns packages in canonical manifest-identity order.
    pub fn packages(&self) -> &[ResolvedPackage] {
        &self.packages
    }

    #[must_use]
    /// Returns canonical `zryna.lock.v1` bytes.
    pub fn lock_bytes(&self) -> &[u8] {
        &self.lock_bytes
    }

    #[must_use]
    /// Returns the domain-separated canonical lock digest.
    pub fn lock_sha256(&self) -> &str {
        &self.lock_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Stable fail-closed package-resolution rejection.
pub struct ResolveError {
    code: &'static str,
    detail: String,
}

impl ResolveError {
    #[must_use]
    /// Constructs a source capability or byte-authentication rejection.
    pub fn source(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4004", detail: detail.into() }
    }

    #[must_use]
    /// Constructs an atomic lock-publication rejection.
    pub fn publication(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4011", detail: detail.into() }
    }

    pub(crate) fn budget(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4001", detail: detail.into() }
    }

    pub(crate) fn wire(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4002", detail: detail.into() }
    }

    pub(crate) fn schema(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4003", detail: detail.into() }
    }

    pub(crate) fn path(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4005", detail: detail.into() }
    }

    pub(crate) fn selection(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4006", detail: detail.into() }
    }

    pub(crate) fn identity(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4007", detail: detail.into() }
    }

    pub(crate) fn graph(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4008", detail: detail.into() }
    }

    pub(crate) fn compatibility(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4009", detail: detail.into() }
    }

    pub(crate) fn frozen(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-P4010", detail: detail.into() }
    }

    #[must_use]
    /// Returns the stable public diagnostic code.
    pub fn code(&self) -> &'static str {
        self.code
    }

    #[must_use]
    /// Returns the bounded non-path diagnostic detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub(crate) fn order(label: &'static str) -> Self {
        Self { code: "ZRYNA-P4003", detail: format!("{label} must be sorted and unique") }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl Error for ResolveError {}

struct Loaded {
    manifest: Manifest,
    instance: PackageInstance,
}

fn load_packages(
    provider: &mut impl PackageSourceProvider,
    root_source: PackageSource,
) -> Result<(PackageSource, BTreeMap<PackageSource, Loaded>), ResolveError> {
    let mut pending = vec![root_source.clone()];
    let mut loaded_by_source = BTreeMap::<PackageSource, Loaded>::new();
    while let Some(source) = pending.pop() {
        if loaded_by_source.contains_key(&source) {
            continue;
        }
        if loaded_by_source.len() == 16 {
            return Err(ResolveError::budget("package count exceeds 16"));
        }
        let material = provider.load(&source)?;
        let manifest: Manifest = parse(&material.manifest)?;
        validation::manifest(&manifest)?;
        if manifest.source != source {
            return Err(ResolveError::source("loaded manifest substitutes the requested source"));
        }
        let id = digest("manifest", &manifest)?;
        let source_sha256 = validation::authenticate_files(&manifest, material.files)?;
        for dependency in manifest.dependencies.iter().rev() {
            pending.push(dependency.source.clone());
        }
        loaded_by_source
            .insert(source, Loaded { manifest, instance: PackageInstance { id, source_sha256 } });
    }
    Ok((root_source, loaded_by_source))
}

/// Resolves and authenticates one complete source-only package graph.
///
/// # Errors
///
/// Returns the first stable rejection and never exposes a partial graph.
pub fn resolve(
    provider: &mut impl PackageSourceProvider,
    root_source: PackageSource,
    mode: LockMode<'_>,
) -> Result<ResolvedGraph, ResolveError> {
    validation::source(&root_source)?;
    let frozen = match mode {
        LockMode::Update => None,
        LockMode::Frozen(input) => {
            let lock: Lock = parse(input)?;
            validate_lock_shape(&lock)?;
            Some(lock)
        }
    };
    let (root_source, loaded_by_source) = load_packages(provider, root_source)?;

    let root_loaded = loaded_by_source
        .get(&root_source)
        .ok_or_else(|| ResolveError::selection("root package was not loaded"))?;
    let compatibility = root_loaded.manifest.compatibility.clone();
    let root = root_loaded.instance.id.clone();
    let mut coordinate_index = BTreeMap::<Vec<u8>, Vec<&Loaded>>::new();
    for loaded in loaded_by_source.values() {
        let key =
            encode(&(&loaded.manifest.name, &loaded.manifest.version, &loaded.manifest.source))?;
        coordinate_index.entry(key).or_default().push(loaded);
        if loaded.manifest.compatibility.compiler != compatibility.compiler
            || loaded.manifest.compatibility.profile != compatibility.profile
            || !compatibility
                .targets
                .iter()
                .all(|target| loaded.manifest.compatibility.targets.contains(target))
        {
            return Err(ResolveError::compatibility(
                "dependency compiler/profile or target coverage differs from the root",
            ));
        }
    }

    let mut packages = Vec::with_capacity(loaded_by_source.len());
    for loaded in loaded_by_source.values() {
        let mut dependencies = Vec::with_capacity(loaded.manifest.dependencies.len());
        for dependency in &loaded.manifest.dependencies {
            let key = encode(&(&dependency.name, &dependency.version, &dependency.source))?;
            let matches = coordinate_index.get(&key).map_or(&[][..], Vec::as_slice);
            if matches.len() != 1 {
                return Err(ResolveError::selection(
                    "exact dependency is absent or has incompatible duplicate instances",
                ));
            }
            dependencies.push((dependency.alias.clone(), matches[0].instance.id.clone()));
        }
        packages.push(ResolvedPackage {
            instance: loaded.instance.clone(),
            name: loaded.manifest.name.clone(),
            version: loaded.manifest.version.clone(),
            source: loaded.manifest.source.clone(),
            dependencies,
        });
    }
    packages.sort_by(|left, right| left.instance.id.cmp(&right.instance.id));
    reject_duplicate_instances(&packages)?;
    reject_cycles_and_orphans(&root, &packages)?;

    let lock = Lock {
        format: "zryna.lock.v1".to_owned(),
        root: root.clone(),
        compatibility: compatibility.clone(),
        packages: packages
            .iter()
            .map(|package| LockPackage {
                id: package.instance.id.clone(),
                source_sha256: package.instance.source_sha256.clone(),
                dependencies: package
                    .dependencies
                    .iter()
                    .map(|(alias, package)| LockEdge {
                        alias: alias.clone(),
                        package: package.clone(),
                    })
                    .collect(),
            })
            .collect(),
    };
    let lock_bytes = encode(&lock)?;
    if let Some(frozen) = frozen
        && frozen != lock
    {
        return Err(ResolveError::frozen("frozen lock differs from the resolved graph"));
    }
    let lock_sha256 = digest("lock", &lock)?;
    Ok(ResolvedGraph { root, compatibility, packages, lock_bytes, lock_sha256 })
}

fn validate_lock_shape(lock: &Lock) -> Result<(), ResolveError> {
    if lock.format != "zryna.lock.v1" || !validation::is_digest(&lock.root) {
        return Err(ResolveError::schema("invalid lock format or root identity"));
    }
    validation::compatibility(&lock.compatibility)?;
    if lock.packages.is_empty() || lock.packages.len() > 16 {
        return Err(ResolveError::budget("lock package count is outside 1..=16"));
    }
    for pair in lock.packages.windows(2) {
        if pair[0].id >= pair[1].id {
            return Err(ResolveError::schema("lock packages must be sorted and unique"));
        }
    }
    for package in &lock.packages {
        if !validation::is_digest(&package.id) || !validation::is_digest(&package.source_sha256) {
            return Err(ResolveError::schema("invalid lock package identity"));
        }
        if package.dependencies.len() > 8 {
            return Err(ResolveError::budget("lock dependency count exceeds 8"));
        }
        for pair in package.dependencies.windows(2) {
            if pair[0].alias >= pair[1].alias {
                return Err(ResolveError::schema("lock edges must be sorted and unique"));
            }
        }
        for edge in &package.dependencies {
            validation::name(&edge.alias)?;
            if !validation::is_digest(&edge.package) {
                return Err(ResolveError::schema("invalid lock edge identity"));
            }
        }
    }
    Ok(())
}

fn reject_duplicate_instances(packages: &[ResolvedPackage]) -> Result<(), ResolveError> {
    for pair in packages.windows(2) {
        if pair[0].instance.id == pair[1].instance.id
            && pair[0].instance.source_sha256 != pair[1].instance.source_sha256
        {
            return Err(ResolveError::identity(
                "one manifest identity has incompatible source instances",
            ));
        }
    }
    Ok(())
}

fn reject_cycles_and_orphans(root: &str, packages: &[ResolvedPackage]) -> Result<(), ResolveError> {
    let by_id: BTreeMap<_, _> =
        packages.iter().map(|package| (package.instance.id.as_str(), package)).collect();
    let mut state = BTreeMap::<&str, u8>::new();
    let mut stack = vec![(root, false)];
    while let Some((id, leaving)) = stack.pop() {
        if leaving {
            state.insert(id, 2);
            continue;
        }
        match state.get(id).copied().unwrap_or(0) {
            1 => return Err(ResolveError::graph("package dependency cycle")),
            2 => continue,
            _ => {}
        }
        let package = by_id
            .get(id)
            .ok_or_else(|| ResolveError::graph("dependency edge selects an absent package"))?;
        state.insert(id, 1);
        stack.push((id, true));
        for (_, child) in package.dependencies.iter().rev() {
            stack.push((child, false));
        }
    }
    if state.values().filter(|&&value| value == 2).count() != packages.len() {
        return Err(ResolveError::graph("unreachable package"));
    }
    Ok(())
}
