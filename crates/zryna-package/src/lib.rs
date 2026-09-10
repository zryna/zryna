//! Closed, deterministic source-package resolution.

#![forbid(unsafe_code)]

mod canonical;
mod identity;
mod model;
mod resolver;
mod validation;

pub use identity::{GraphRole, PackageInstance, PackageSemanticDomain};
pub use model::{Compatibility, PackageSource, PackageSourceKind};
pub use resolver::{
    LockMode, PackageFile, PackageMaterial, PackageSourceProvider, ResolveError, ResolvedGraph,
    ResolvedPackage, resolve,
};

/// Validates one closed package source tuple without opening material.
///
/// # Errors
///
/// Returns a stable source or path rejection for unsupported spelling.
pub fn validate_source(source: &PackageSource) -> Result<(), ResolveError> {
    validation::source(source)
}
