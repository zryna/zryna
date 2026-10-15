//! Read-only identities from the original authenticated declaration set.
use super::{VerifiedDeclarationSet, VerifiedOperation};
use zryna_syntax::native_c_v0::raw::Direction;

impl VerifiedDeclarationSet {
    /// Exact versioned ids of all independently captured libraries.
    #[must_use]
    pub fn library_ids(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.libraries.iter().map(|library| library.id.as_str())
    }
    /// Canonical keys for all verified imports and exports, including unused declarations.
    #[must_use]
    pub fn operation_keys(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.declarations.operations.iter().map(|operation| operation.key.as_str())
    }
}
impl VerifiedOperation<'_> {
    /// Exact external symbol, without case folding or inferred aliases.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.operation.symbol
    }
    /// Whether this record binds an original source-defined export implementation.
    #[must_use]
    pub fn is_export(&self) -> bool {
        self.operation.direction == Direction::Export
    }
    /// Compares every contract field while excluding only source-binding location/ordinal.
    /// Export callers must independently compare the original source issuer and span.
    #[must_use]
    pub fn same_contract(&self, other: VerifiedOperation<'_>) -> bool {
        let mut record = self.operation.clone();
        record.source_binding = other.operation.source_binding.clone();
        record == *other.operation
    }
}
