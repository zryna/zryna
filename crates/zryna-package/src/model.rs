use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
/// Closed source kinds admitted by package contract v1.
pub enum PackageSourceKind {
    /// One prepopulated canonical HTTPS repository at an exact commit.
    Git,
    /// One package locator relative to the declared source root.
    Local,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
/// Exact source tuple used for dependency selection.
pub struct PackageSource {
    /// Source acquisition class.
    pub kind: PackageSourceKind,
    /// Canonical local or HTTPS Git locator.
    pub locator: String,
    /// Empty for local sources or one full lowercase Git commit.
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Exact compiler, language-profile, and target compatibility claim.
pub struct Compatibility {
    /// Exact compatible compiler version.
    pub compiler: String,
    /// Exact compatible language profile.
    pub profile: String,
    /// Sorted target coverage.
    pub targets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Checksum {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Dependency {
    pub alias: String,
    pub name: String,
    pub version: String,
    pub source: PackageSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub format: String,
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    pub compatibility: Compatibility,
    pub files: Vec<Checksum>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LockEdge {
    pub alias: String,
    pub package: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LockPackage {
    pub id: String,
    #[serde(rename = "sourceSha256")]
    pub source_sha256: String,
    pub dependencies: Vec<LockEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Lock {
    pub format: String,
    pub root: String,
    pub compatibility: Compatibility,
    pub packages: Vec<LockPackage>,
}
