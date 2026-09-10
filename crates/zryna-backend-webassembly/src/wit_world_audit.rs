//! Authenticated resolution and independent audit for the pinned M4 WIT worlds.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use wit_parser::{Resolve, SourceMap, WorldItem};
use zryna_diagnostics::Diagnostic;

mod browser;
mod command;
mod pins;

pub(crate) use browser::{AuthenticatedBrowserWorld, BROWSER_WORLD};
pub(crate) use command::AuthenticatedCommandWorld;

const MAX_SOURCE_FILES: usize = 34;
const MAX_SOURCE_BYTES: usize = 32 * 1024;
const MAX_ROOT_SOURCE_BYTES: usize = 16 * 1024;
const MAX_TOTAL_SOURCE_BYTES: usize = 256 * 1024;
const MAX_SOURCE_PATH_BYTES: usize = 96;
const MAX_RESOLVED_PACKAGES: usize = 8;
const MAX_RESOLVED_INTERFACES: usize = 32;
const MAX_RESOLVED_WORLDS: usize = 16;
const MAX_RESOLVED_TYPES: usize = 4_096;

/// One named WIT source presented to the pinned dependency audit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitSource {
    path: String,
    bytes: Vec<u8>,
}

impl WitSource {
    /// Creates an owned input. The path is a logical portable identity and is never opened.
    #[must_use]
    pub fn new(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self { path: path.into(), bytes: bytes.into() }
    }

    /// Returns the logical source path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact source bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One independently resolved world observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedWitWorld {
    identity: String,
    explicit_imports: Vec<String>,
    resolved_imports: Vec<String>,
    exports: Vec<String>,
}

impl ResolvedWitWorld {
    /// Returns the exact package-qualified world identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the canonical, sorted imports declared explicitly by the accepted world contract.
    #[must_use]
    pub fn explicit_imports(&self) -> &[String] {
        &self.explicit_imports
    }

    /// Returns canonical, sorted imports after parser-mandated type-dependency elaboration.
    #[must_use]
    pub fn resolved_imports(&self) -> &[String] {
        &self.resolved_imports
    }

    /// Returns canonical, sorted interface exports.
    #[must_use]
    pub fn exports(&self) -> &[String] {
        &self.exports
    }
}

/// Opaque evidence that all pinned sources resolved and every accepted world matched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitWorldAudit {
    packages: Vec<String>,
    worlds: Vec<ResolvedWitWorld>,
}

impl WitWorldAudit {
    /// Returns all exact resolved package identities in canonical order.
    #[must_use]
    pub fn packages(&self) -> &[String] {
        &self.packages
    }

    /// Returns the three accepted world observations in browser, command, server order.
    #[must_use]
    pub fn worlds(&self) -> &[ResolvedWitWorld] {
        &self.worlds
    }
}

/// Authenticates, parses, resolves and independently audits the accepted WIT source closure.
///
/// The input must contain the exact local world source and the complete WASI 0.2.12 source set
/// pinned by Issue #383. Input order is ignored. This function creates a fresh resolver for every
/// call and neither emits nor instantiates a component.
///
/// # Errors
///
/// Returns a stable diagnostic for resource overflow, missing or substituted source, parser or
/// dependency failure, or any resolved package/world/interface drift.
pub fn audit_pinned_wit_worlds(sources: &[WitSource]) -> Result<WitWorldAudit, Diagnostic> {
    let authenticated = authenticate(sources)?;
    let (resolve, root) = resolve_sources(&authenticated)?;
    audit_resolved(&resolve, root)
}

/// Returns the exact reviewed WIT source closure embedded in this backend build.
///
/// Callers may pass this immutable source set to component emission without reading ambient
/// filesystem state. The ordinary audit entrypoint remains available for independently supplied
/// and hostile inputs.
#[must_use]
pub fn pinned_wit_sources() -> Vec<WitSource> {
    pins::SOURCES
        .iter()
        .zip(pins::SOURCE_BYTES)
        .map(|(pin, bytes)| WitSource::new(pin.path, *bytes))
        .collect()
}

fn authenticate(
    sources: &[WitSource],
) -> Result<Vec<(&'static pins::SourcePin, &str)>, Diagnostic> {
    if sources.len() > MAX_SOURCE_FILES {
        return Err(budget_error("source count", MAX_SOURCE_FILES));
    }

    let total_bytes = sources.iter().try_fold(0_usize, |total, source| {
        total
            .checked_add(source.bytes.len())
            .ok_or_else(|| budget_error("total source bytes", MAX_TOTAL_SOURCE_BYTES))
    })?;
    if total_bytes > MAX_TOTAL_SOURCE_BYTES {
        return Err(budget_error("total source bytes", MAX_TOTAL_SOURCE_BYTES));
    }
    if sources.iter().any(|source| source.path.len() > MAX_SOURCE_PATH_BYTES) {
        return Err(budget_error("source path bytes", MAX_SOURCE_PATH_BYTES));
    }

    let mut ordered = sources.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.path.cmp(&right.path));
    let mut seen = BTreeSet::new();
    let mut authenticated = Vec::with_capacity(ordered.len());
    for source in ordered {
        let limit = if source.path == pins::SOURCES[0].path {
            MAX_ROOT_SOURCE_BYTES
        } else {
            MAX_SOURCE_BYTES
        };
        if source.bytes.len() > limit {
            return Err(budget_error("single source bytes", limit));
        }
        if !seen.insert(source.path.as_str()) {
            return Err(authentication_error(format!("duplicate WIT source `{}`", source.path)));
        }
        let pin = pins::SOURCES
            .iter()
            .find(|pin| pin.path == source.path)
            .ok_or_else(|| authentication_error(format!("unknown WIT source `{}`", source.path)))?;
        let actual = Sha256::digest(&source.bytes);
        if format!("{actual:x}") != pin.sha256 {
            return Err(authentication_error(format!("substituted WIT source `{}`", source.path)));
        }
        let text = std::str::from_utf8(&source.bytes)
            .map_err(|_| parse_error(format!("WIT source `{}` is not UTF-8", source.path)))?;
        authenticated.push((pin, text));
    }

    if authenticated.len() != pins::SOURCES.len() {
        let missing = pins::SOURCES
            .iter()
            .find(|pin| !seen.contains(pin.path))
            .map_or("unknown", |pin| pin.path);
        return Err(authentication_error(format!("missing WIT source `{missing}`")));
    }
    Ok(authenticated)
}

fn resolve_sources(
    sources: &[(&pins::SourcePin, &str)],
) -> Result<(Resolve, wit_parser::PackageId), Diagnostic> {
    resolve_source_texts(sources.iter().map(|(pin, text)| (pin.package, pin.path, *text)))
}

fn resolve_source_texts<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> Result<(Resolve, wit_parser::PackageId), Diagnostic> {
    let mut maps = BTreeMap::<&str, SourceMap>::new();
    for (package, path, text) in sources {
        maps.entry(package).or_default().push_str(path, text);
    }

    let mut root = None;
    let mut dependencies = Vec::with_capacity(maps.len().saturating_sub(1));
    for (expected_package, map) in maps {
        let group = map.parse().map_err(|(_, error)| {
            parse_error(format!("failed to parse `{expected_package}`: {error}"))
        })?;
        if !group.nested.is_empty() || group.main.name.to_string() != expected_package {
            return Err(audit_error(format!("resolved unexpected package `{}`", group.main.name)));
        }
        if expected_package == pins::ROOT_PACKAGE {
            root = Some(group);
        } else {
            dependencies.push(group);
        }
    }

    let root = root.ok_or_else(|| authentication_error("missing local WIT package"))?;
    let mut resolve = Resolve::default();
    let root_id = resolve.push_groups(root, dependencies).map_err(|error| {
        parse_error(format!("failed to resolve pinned WIT dependency graph: {error}"))
    })?;
    enforce_resolved_budgets(&resolve)?;
    Ok((resolve, root_id))
}

fn enforce_resolved_budgets(resolve: &Resolve) -> Result<(), Diagnostic> {
    for (actual, limit, name) in [
        (resolve.packages.len(), MAX_RESOLVED_PACKAGES, "resolved packages"),
        (resolve.interfaces.len(), MAX_RESOLVED_INTERFACES, "resolved interfaces"),
        (resolve.worlds.len(), MAX_RESOLVED_WORLDS, "resolved worlds"),
        (resolve.types.len(), MAX_RESOLVED_TYPES, "resolved types"),
    ] {
        if actual > limit {
            return Err(budget_error(name, limit));
        }
    }
    Ok(())
}

fn audit_resolved(
    resolve: &Resolve,
    root: wit_parser::PackageId,
) -> Result<WitWorldAudit, Diagnostic> {
    let mut packages = resolve.package_names.keys().map(ToString::to_string).collect::<Vec<_>>();
    packages.sort();
    if packages.iter().map(String::as_str).ne(pins::PACKAGES.iter().copied()) {
        return Err(audit_error("resolved package identities or versions drifted"));
    }

    let mut worlds = Vec::with_capacity(pins::WORLDS.len());
    for expected in pins::WORLDS {
        let world_id = resolve.select_world(&[root], Some(expected.identity)).map_err(|error| {
            audit_error(format!("failed to select `{}`: {error}", expected.identity))
        })?;
        let world = &resolve.worlds[world_id];
        let package = world.package.ok_or_else(|| audit_error("resolved world has no package"))?;
        let identity = resolve.id_of_name(package, &world.name);
        if identity != expected.identity || !world.includes.is_empty() {
            return Err(audit_error(format!(
                "resolved world `{}` broadened or changed identity",
                expected.identity
            )));
        }
        let resolved_imports = interface_ids(resolve, world.imports.values(), "import")?;
        let exports = interface_ids(resolve, world.exports.values(), "export")?;
        if resolved_imports.iter().map(String::as_str).ne(expected.resolved_imports.iter().copied())
            || exports.iter().map(String::as_str).ne(expected.exports.iter().copied())
        {
            return Err(audit_error(format!(
                "resolved interfaces drifted for `{}`",
                expected.identity
            )));
        }
        worlds.push(ResolvedWitWorld {
            identity,
            explicit_imports: expected.explicit_imports.iter().map(ToString::to_string).collect(),
            resolved_imports,
            exports,
        });
    }
    Ok(WitWorldAudit { packages, worlds })
}

fn interface_ids<'a>(
    resolve: &Resolve,
    items: impl Iterator<Item = &'a WorldItem>,
    role: &str,
) -> Result<Vec<String>, Diagnostic> {
    let mut ids = Vec::new();
    for item in items {
        let WorldItem::Interface { id, .. } = item else {
            return Err(audit_error(format!("resolved world contains a non-interface {role}")));
        };
        ids.push(resolve.id_of(*id).ok_or_else(|| audit_error(format!("anonymous world {role}")))?);
    }
    ids.sort();
    Ok(ids)
}

fn parse_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error("ZRYNA-W4000", None, message, "restore the exact pinned WIT source closure")
}

fn authentication_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error("ZRYNA-W4001", None, message, "restore every source from its reviewed hash")
}

fn audit_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4002",
        None,
        message,
        "use the accepted package, world, version and interface identities",
    )
}

fn budget_error(resource: &str, limit: usize) -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-W4003",
        None,
        format!("WIT {resource} exceeds the limit of {limit}"),
        "reduce the input before parsing or resolution",
    )
}

#[cfg(test)]
mod tests;
