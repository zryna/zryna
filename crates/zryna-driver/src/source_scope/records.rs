//! Owned provenance rows; none of these read-only records is source or executable authority.
use crate::{CommandFailure, generic_closure::VerifiedGenericClosure};
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericSourceModule {
    id: u32,
    path: String,
    bytes: u64,
    sha256: [u8; 32],
}
impl GenericSourceModule {
    pub(crate) const fn id(&self) -> u32 {
        self.id
    }
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
    pub(crate) const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub(crate) const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericSourceEdge {
    importer: String,
    target: String,
    specifier: String,
    imported: String,
    local: String,
}
impl GenericSourceEdge {
    pub(crate) fn importer(&self) -> &str {
        &self.importer
    }
    pub(crate) fn target(&self) -> &str {
        &self.target
    }
    pub(crate) fn specifier(&self) -> &str {
        &self.specifier
    }
    pub(crate) fn imported(&self) -> &str {
        &self.imported
    }
    pub(crate) fn local(&self) -> &str {
        &self.local
    }
}
pub(super) fn capture(
    closure: &VerifiedGenericClosure,
) -> Result<(Vec<GenericSourceModule>, Vec<GenericSourceEdge>), CommandFailure> {
    let mut modules = super::failure::reserve(closure.modules().len())?;
    for row in closure.modules() {
        let id =
            closure.sources().verify_file_id(row.id()).map_err(|_| super::failure::invariant())?;
        let source = closure.sources().source(id).ok_or_else(super::failure::invariant)?;
        let digest: [u8; 32] = Sha256::digest(source.text().as_bytes()).into();
        if source.path() != row.path() || digest != *row.source_sha256() {
            return Err(super::failure::invariant());
        }
        modules.push(GenericSourceModule {
            id: row.id(),
            path: row.path().as_str().into(),
            bytes: u64::try_from(source.text().len()).map_err(|_| super::failure::invariant())?,
            sha256: digest,
        });
    }
    let mut edges = super::failure::reserve(closure.edges().len())?;
    for row in closure.edges() {
        edges.push(GenericSourceEdge {
            importer: row.importer().as_str().into(),
            target: row.target().as_str().into(),
            specifier: row.specifier().into(),
            imported: row.imported().into(),
            local: row.local().into(),
        });
    }
    Ok((modules, edges))
}
