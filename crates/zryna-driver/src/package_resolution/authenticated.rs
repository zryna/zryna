use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};
use zryna_package::{PackageFile, PackageSource, ResolveError, ResolvedGraph};

#[derive(Clone, Debug, Eq, PartialEq)]
/// One authenticated source file retained as immutable bytes for later compilation.
pub struct AuthenticatedPackageFile {
    path: String,
    bytes: Vec<u8>,
    sha256: String,
}

impl AuthenticatedPackageFile {
    #[must_use]
    /// Returns the portable package-relative path.
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    /// Returns the exact bytes authenticated during package resolution.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    /// Returns the raw SHA-256 digest of the retained bytes.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Complete authenticated source inventory for one resolved package instance.
pub struct AuthenticatedPackageSources {
    package_id: String,
    source_sha256: String,
    files: Vec<AuthenticatedPackageFile>,
}

impl AuthenticatedPackageSources {
    #[must_use]
    /// Returns the package manifest identity.
    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    #[must_use]
    /// Returns the package's domain-separated complete source-inventory digest.
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    #[must_use]
    /// Returns files in canonical package-relative path order.
    pub fn files(&self) -> &[AuthenticatedPackageFile] {
        &self.files
    }
}

pub(super) fn authenticated_sources(
    graph: &ResolvedGraph,
    loaded: &BTreeMap<PackageSource, Vec<PackageFile>>,
) -> Result<Vec<AuthenticatedPackageSources>, ResolveError> {
    graph
        .packages()
        .iter()
        .map(|package| {
            let files = loaded.get(package.source()).ok_or_else(|| {
                ResolveError::source("authenticated package source inventory is unavailable")
            })?;
            Ok(AuthenticatedPackageSources {
                package_id: package.instance().manifest_id().to_owned(),
                source_sha256: package.instance().source_sha256().to_owned(),
                files: files
                    .iter()
                    .map(|file| AuthenticatedPackageFile {
                        path: file.path.clone(),
                        sha256: format!("{:x}", Sha256::digest(&file.bytes)),
                        bytes: file.bytes.clone(),
                    })
                    .collect(),
            })
        })
        .collect()
}
