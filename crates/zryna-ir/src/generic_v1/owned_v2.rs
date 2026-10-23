//! Separate source-bound owned/loan/drop successor; frozen Copy bytes remain unchanged.
use super::{Failure, raw as graph, reject, source::Originals};
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};
use zryna_ownership_runtime_abi::generic_v1::VerifiedOwnershipRuntimeAbi;
use zryna_source::{FileId, SourceMap};
use zryna_syntax::v5::VerifiedProjectSyntaxV5;
/// Private admission chosen only by authenticated issuer entrypoints.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Admission {
    #[default]
    LegacyOwned,
    StructV4,
}
mod extension_index;
mod originals;
mod plan;
mod producer;
pub mod raw;
mod source;
mod typed;
pub mod wire;
pub use originals::{check_original_bodies, check_original_bodies_v4};
pub use producer::{produce_claim, produce_claim_v4};
/// Immutable exact source/layout/runtime/ABI-bound owned program.
/// ```compile_fail
/// fn forge(raw: zryna_ir::generic_v1::owned_v2::raw::Program) {
///     let _: zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram<'_> = raw;
/// }
/// ```
#[derive(Debug)]
pub struct VerifiedOwnedProgram<'a> {
    claim: raw::Program,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
    abi: zryna_abi::VerifiedScalarAbiModule,
    exports: Vec<usize>,
    extension_index: Option<extension_index::ExtensionIndex>,
}
impl<'a> VerifiedOwnedProgram<'a> {
    /// Complete authenticated functions in unsigned key order.
    #[must_use]
    pub fn functions(&self) -> &[graph::Function] {
        &self.claim.graph.functions
    }
    /// Exact retained successor layouts.
    #[must_use]
    pub const fn layouts(&self, target: StorageTarget) -> &'a VerifiedLayouts {
        match target {
            StorageTarget::Linear32V1 => self.linear,
            StorageTarget::LinuxX8664V1 => self.linux,
        }
    }
    /// Exact retained runtime declarations.
    #[must_use]
    pub const fn runtime(&self) -> &'a VerifiedOwnershipRuntimeAbi<'a> {
        self.runtime
    }
    /// Unchanged scalar export ABI.
    #[must_use]
    pub const fn scalar_abi(&self) -> &zryna_abi::VerifiedScalarAbiModule {
        &self.abi
    }
    /// Exact function indices in export declaration order.
    #[must_use]
    pub fn export_functions(&self) -> &[usize] {
        &self.exports
    }
    /// Verified separate operation for one result ID.
    #[must_use]
    pub fn extension(&self, function: usize, result: u32) -> Option<&raw::Operation> {
        if let Some(index) = &self.extension_index {
            let ordinal = usize::try_from(index.ordinal(function, result)?).ok()?;
            let extension = self.claim.extensions.get(function)?.get(ordinal)?;
            #[cfg(test)]
            extension_lookup_tests::inspect_candidate();
            return Some(&extension.operation);
        }
        self.claim
            .extensions
            .get(function)?
            .iter()
            .find(|e| {
                #[cfg(test)]
                extension_lookup_tests::inspect_candidate();
                e.result == result
            })
            .map(|e| &e.operation)
    }
    /// Complete immutable cleanup plan.
    #[must_use]
    pub fn plan(&self, function: usize) -> Option<&raw::Plan> {
        self.claim.plans.get(function)
    }
}
/// Verifies wire, original opaque bodies, typed CFG and complete owner/loan/drop plans.
/// # Errors
/// Rejects foreign authorities, fabricated effects, moved/loaned use, escapes and cleanup drift.
pub fn verify<'a>(
    decoded: wire::DecodedProgram,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
) -> Result<VerifiedOwnedProgram<'a>, Failure> {
    verify_claim(
        (decoded.0, Admission::LegacyOwned),
        syntax,
        sources,
        entry,
        linear,
        linux,
        runtime,
    )
}
/// Verifies the distinct private Struct transport against independent source, layouts and plans.
/// # Errors
/// Rejects foreign authorities, unproved fields, moved/loaned uses and cleanup drift.
pub fn verify_v4<'a>(
    decoded: wire::v4::DecodedOwnedV4,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
) -> Result<VerifiedOwnedProgram<'a>, Failure> {
    verify_claim((decoded.0, Admission::StructV4), syntax, sources, entry, linear, linux, runtime)
}
fn verify_claim<'a>(
    (claim, admission): (raw::Program, Admission),
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
) -> Result<VerifiedOwnedProgram<'a>, Failure> {
    if !syntax.is_bound_to(sources)
        || sources.verify_file_id(entry.index()).ok() != Some(entry)
        || !runtime.is_bound_to(linear, linux)
    {
        return Err(reject("foreign owned successor authority"));
    }
    let inventory = super::inventory::check(&claim.graph, sources, linear, linux)?;
    let originals = Originals::check(&claim.graph, syntax)?;
    super::copy_v1::entry_closure(&originals, entry.index())?;
    super::substitution::check(&claim.graph, linear, &originals)?;
    super::source_calls::check(&claim.graph, &originals)?;
    source::check_for(&claim, &originals, sources, admission)?;
    super::source_body::demand(&claim.graph)?;
    match admission {
        Admission::LegacyOwned => super::copy_v1::check_type_demand(&claim.graph, linear)?,
        Admission::StructV4 => {
            let roots = super::source_demand::derive(&claim.graph, &originals)?;
            super::copy_v1::check_type_demand_with_roots(&claim.graph, linear, &roots)?;
        }
    }
    match admission {
        Admission::LegacyOwned => typed::check(&claim, sources, linear, &inventory)?,
        Admission::StructV4 => typed::check_for(&claim, sources, linear, &inventory, admission)?,
    }
    let (plans, spent) = match admission {
        Admission::LegacyOwned => (plan::derive(&claim.graph, &claim.extensions, linear)?, None),
        Admission::StructV4 => {
            let (plans, used) = plan::derive_counted(&claim.graph, &claim.extensions, linear)?;
            (plans, Some(used))
        }
    };
    if plans != claim.plans {
        return Err(reject("owned cleanup/loan/failure plan is not exact"));
    }
    super::source_abi::check(&claim.graph, &originals, sources, entry.index())?;
    let (abi, exports) = super::copy_v1::scalar_abi(&claim.graph, entry.index())?;
    let extension_index = if let Some(mut used) = spent {
        Some(extension_index::ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used)?)
    } else {
        None
    };
    Ok(VerifiedOwnedProgram { claim, linear, linux, runtime, abi, exports, extension_index })
}

#[cfg(doctest)]
mod struct_v4_type_contract;

#[cfg(test)]
mod original_cycle_tests;

#[cfg(test)]
mod extension_lookup_tests;
