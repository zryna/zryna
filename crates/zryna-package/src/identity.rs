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
/// Compiler-facing nominal coordinate retained from one authenticated graph.
pub struct NominalIdentity {
    role: GraphRole,
    profile: String,
    package: PackageInstance,
    module: String,
    declaration_ordinal: u32,
}

impl ResolvedGraph {
    /// Constructs a nominal coordinate for a package and portable module in this graph.
    ///
    /// # Errors
    ///
    /// Returns an identity error if the package is absent or the module path is invalid.
    pub fn nominal_identity(
        &self,
        package_id: &str,
        module: &str,
        declaration_ordinal: u32,
    ) -> Result<NominalIdentity, ResolveError> {
        let package = self
            .packages()
            .iter()
            .find(|package| package.instance().manifest_id() == package_id)
            .ok_or_else(|| ResolveError::identity("nominal package is absent from the graph"))?;
        crate::validation::portable_path(module)?;
        Ok(NominalIdentity {
            role: GraphRole::TargetRuntime,
            profile: self.compatibility().profile.clone(),
            package: package.instance().clone(),
            module: module.to_owned(),
            declaration_ordinal,
        })
    }
}
