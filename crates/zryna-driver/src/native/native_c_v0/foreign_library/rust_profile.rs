//! Distinct source-bound Rust-shaped ELF data; neither compiler origin nor execution authority.

use super::{
    ForeignLibraryInput, HandleLinkRequirements, check_requirements_binding, checked_material,
    identity_error,
};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeSet, sync::Arc};
use zryna_diagnostics::Diagnostic;

mod tables;

/// Actual independent foreign-source and artifact expectations, not a compilation issuer.
/// The declaration issuer authenticates Zryna source/header/policy only.
pub struct RustProfileLibraryInput<'a> {
    /// Original library/header/policy and exact independently expected ELF bytes.
    pub library: ForeignLibraryInput<'a>,
    /// Independently retained UTF-8 foreign Rust source data, not verified Rust syntax.
    pub source_bytes: &'a [u8],
    /// Independently expected source byte length.
    pub source_size: usize,
    /// Independently expected source digest; it proves no compilation relationship.
    pub source_sha256: &'a [u8; 32],
}

/// Immutable Rust-profile artifact data bound to genuine original Zryna issuers.
///
/// Neither the metadata nor matching source/object expectations prove that Rust compiled
/// these bytes. This type cannot run, construct a compiler invocation, convert to the GCC
/// capture, authorize a recipe, or replace retained host/provider qualification.
#[derive(Clone, Debug)]
pub struct CapturedRustProfileLibrary {
    requirements: HandleLinkRequirements,
    library_id: String,
    bytes: Arc<[u8]>,
    source: Arc<[u8]>,
    object_sha256: [u8; 32],
    source_sha256: [u8; 32],
    definitions: BTreeSet<String>,
}

impl CapturedRustProfileLibrary {
    /// Exact original source/machine/library requirement capability.
    #[must_use]
    pub const fn requirements(&self) -> &HandleLinkRequirements {
        &self.requirements
    }
    /// Independently captured artifact bytes, with no linker or runtime permission.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Independent foreign-source data; not authenticated compilation provenance.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    /// Exact original declaration library identity.
    #[must_use]
    pub fn library_id(&self) -> &str {
        &self.library_id
    }
    /// Original independently expected acquired-object digest.
    #[must_use]
    pub const fn object_sha256(&self) -> &[u8; 32] {
        &self.object_sha256
    }
    /// Original independently expected source-data digest.
    #[must_use]
    pub const fn source_sha256(&self) -> &[u8; 32] {
        &self.source_sha256
    }
    /// Complete declared strong function definitions, including unselected operations.
    pub fn definitions(&self) -> impl ExactSizeIterator<Item = &str> {
        self.definitions.iter().map(String::as_str)
    }
    /// Checks this capture's original issuer, not a globally privileged first issuer.
    /// # Errors
    /// Rejects independent reissue or changed selected object/declaration/library requirements.
    pub fn check_binding(&self, requirements: &HandleLinkRequirements) -> Result<(), Diagnostic> {
        check_requirements_binding(&self.requirements, requirements)?;
        if !requirements.libraries().iter().any(|library| library.id() == self.library_id) {
            return Err(identity_error());
        }
        Ok(())
    }
}

/// Captures a distinct closed Rust-shaped ELF profile as data only.
///
/// A separately genuine requirements capability may produce its own separate capture.
/// Both retained sources and objects are bounded before hashing or immutable copying.
/// No equality of digests, compiler comment or filename reconstructs a compilation issuer.
/// # Errors
/// Rejects substituted material, missing libraries, open ELF profiles and resource limits.
pub fn capture_rust_profile_library(
    requirements: &HandleLinkRequirements,
    input: &RustProfileLibraryInput<'_>,
) -> Result<CapturedRustProfileLibrary, Diagnostic> {
    let maximum = zryna_backend_native::MAX_NATIVE_OBJECT_BYTES;
    if input.source_bytes.len() > maximum || input.library.object_bytes.len() > maximum {
        return Err(super::audit::limit_error());
    }
    if input.source_bytes.is_empty()
        || input.source_size != input.source_bytes.len()
        || &<[u8; 32]>::from(Sha256::digest(input.source_bytes)) != input.source_sha256
        || std::str::from_utf8(input.source_bytes).is_err()
    {
        return Err(identity_error());
    }
    let definitions = checked_material(requirements, &input.library)?;
    if !input.library.dependency_symbols.is_empty() {
        return Err(super::audit::audit_error());
    }
    tables::check(input.library.object_bytes, &definitions)?;
    Ok(CapturedRustProfileLibrary {
        requirements: requirements.clone(),
        library_id: input.library.library_id.to_owned(),
        bytes: Arc::from(input.library.object_bytes),
        source: Arc::from(input.source_bytes),
        object_sha256: *input.library.object_sha256,
        source_sha256: *input.source_sha256,
        definitions,
    })
}
