use crate::resolver::{ResolveError, ResolvedGraph};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Role-scoped semantic domain for a resolved package graph.
pub enum GraphRole {
    /// Source packages whose verified code enters the target output closure.
    TargetRuntime,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Exact authenticated manifest and source-inventory identity pair.
pub struct PackageInstance {
    pub(crate) id: String,
    pub(crate) source_sha256: String,
}

impl PackageInstance {
    #[must_use]
    /// Returns the domain-separated canonical manifest digest.
    pub fn manifest_id(&self) -> &str {
        &self.id
    }

    #[must_use]
    /// Returns the domain-separated complete source-inventory digest.
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Opaque package-scoped input to later semantics-owned nominal identity.
pub struct PackageSemanticDomain {
    role: GraphRole,
    profile: String,
    package: PackageInstance,
}

impl PackageSemanticDomain {
    #[must_use]
    /// Returns the exact role of the authenticated package graph.
    pub const fn role(&self) -> GraphRole {
        self.role
    }

    #[must_use]
    /// Returns the exact selected language profile.
    pub fn profile(&self) -> &str {
        &self.profile
    }

    #[must_use]
    /// Returns the exact authenticated package instance.
    pub const fn package(&self) -> &PackageInstance {
        &self.package
    }
}

impl ResolvedGraph {
    /// Returns the package-scoped semantic domain authenticated by this graph.
    ///
    /// This is not a nominal declaration identity. Only later semantics may combine it with an
    /// authenticated module and a real source-ordered declaration ordinal.
    ///
    /// # Errors
    ///
    /// Returns an identity error if the package is absent from this graph.
    pub fn package_semantic_domain(
        &self,
        package_id: &str,
    ) -> Result<PackageSemanticDomain, ResolveError> {
        let package = self
            .packages()
            .iter()
            .find(|package| package.instance().manifest_id() == package_id)
            .ok_or_else(|| {
                ResolveError::identity("semantic-domain package is absent from the graph")
            })?;
        Ok(PackageSemanticDomain {
            role: GraphRole::TargetRuntime,
            profile: self.compatibility().profile.clone(),
            package: package.instance().clone(),
        })
    }
}
