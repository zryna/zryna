//! Independent source-bound execution domain for the accepted bounded-generics contract.
//!
//! This internal implementation is incomplete and does not activate a public driver profile.
//! Wire admission, source replay and each owning check precede executable authority.
//! Older Copy and owned domains retain their exact APIs, bytes and meanings.

use super::{Failure, budget as resource_failure, reject, reserve};
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};
use zryna_ownership_runtime_abi::generic_v1::VerifiedOwnershipRuntimeAbi;
use zryna_source::{FileId, SourceMap};
use zryna_syntax::v5::VerifiedProjectSyntaxV5;

mod abi;
mod budget;
mod frontier;
pub mod invocation;
mod places;
mod plan;
pub mod raw;
mod source;
mod state;
mod type_world;
mod typed;
mod views;
pub mod wire;

pub use views::{
    CloneNodeKind, OperationSite, VerifiedBlock, VerifiedCleanupAction, VerifiedCleanupPlan,
    VerifiedClone, VerifiedCloneNode, VerifiedCloneSource, VerifiedDropTarget, VerifiedDropTargets,
    VerifiedFunction, VerifiedIndexCopy, VerifiedIndexedLoan, VerifiedLoan, VerifiedLoanOrigin,
    VerifiedOperation, VerifiedParameter, VerifiedPlace, VerifiedSharedConstruct,
    VerifiedUpgradeEdge, VerifiedValue, VerifiedVecConstruct, VerifiedVecPush,
    VerifiedWeakDowngrade, VerifiedWeakUpgrade,
};

/// One independently source/type/state/cleanup/runtime-bound execution program.
///
/// Raw claims and older seals cannot construct this authority.
/// ```compile_fail
/// fn forge(raw: zryna_ir::generic_v1::execution_v1::raw::Program) {
///     let _: zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_> = raw;
/// }
/// ```
#[derive(Debug)]
pub struct VerifiedExecution<'a> {
    program: raw::Program,
    source: source::SourceProof,
    typed: typed::Typed,
    plan: plan::Plan,
    entry: FileId,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
    abi: zryna_abi::VerifiedScalarAbiModule,
    exports: Vec<usize>,
}

impl<'a> VerifiedExecution<'a> {
    /// The exact independently selected entry source file.
    #[must_use]
    pub const fn entry_file(&self) -> FileId {
        self.entry
    }
    /// Exact retained successor layouts for the selected storage target.
    #[must_use]
    pub const fn layouts(&self, target: StorageTarget) -> &'a VerifiedLayouts {
        match target {
            StorageTarget::Linear32V1 => self.linear,
            StorageTarget::LinuxX8664V1 => self.linux,
        }
    }
    /// The same independently verified successor runtime declarations.
    #[must_use]
    pub const fn runtime(&self) -> &'a VerifiedOwnershipRuntimeAbi<'a> {
        self.runtime
    }
    /// Unchanged scalar public ABI authority for nongeneric entry wrappers only.
    #[must_use]
    pub const fn scalar_abi(&self) -> &zryna_abi::VerifiedScalarAbiModule {
        &self.abi
    }
    /// Same-program function indices in the scalar ABI's declaration order.
    #[must_use]
    pub fn export_functions(&self) -> &[usize] {
        &self.exports
    }
}

/// Checks every original body, including unused opaque templates, before discovery.
/// # Errors
/// Returns owning source diagnostics or bounded infrastructure failures without authority.
pub fn check_original_bodies(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<(), Failure> {
    source::check_original_bodies(syntax, sources)
}

/// Produces untrusted claims using the actual selected entry and complete generic keys.
/// # Errors
/// Rejects foreign source/layout/entry authority, original illegality and amplification limits.
pub fn produce_claim(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    generic_keys: &[&[u8]],
) -> Result<raw::Program, Failure> {
    if !syntax.is_bound_to(sources) || sources.verify_file_id(entry.index()).ok() != Some(entry) {
        return Err(reject("foreign full-profile producer source or entry"));
    }
    source::produce_claim(syntax, sources, entry, linear, linux, generic_keys)
}

/// Independently seals the new carrier after complete source and owning-phase checks.
/// # Errors
/// Rejects foreign authority, inexact source/type/CFG/ownership/cleanup claims and budgets.
pub fn verify<'a>(
    decoded: wire::DecodedExecutionV1,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &'a VerifiedLayouts,
    linux: &'a VerifiedLayouts,
    runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
) -> Result<VerifiedExecution<'a>, Failure> {
    let program = decoded.0;
    if !syntax.is_bound_to(sources)
        || sources.verify_file_id(entry.index()).ok() != Some(entry)
        || program.entry_module != entry.index()
        || linear.source_map_identity() != sources.identity()
        || linux.source_map_identity() != sources.identity()
        || !runtime.is_bound_to(linear, linux)
    {
        return Err(reject("foreign full-profile source, entry, layouts or runtime authority"));
    }
    let mut spent = 0;
    let source =
        source::verify_source(&program, syntax, sources, entry, linear, linux, &mut spent)?;
    let typed = typed::check(&program, &source, linear, linux, &mut spent)?;
    let plan = plan::derive(&program, &typed, &mut spent)?;
    let (abi, exports) = abi::derive(&program, syntax, sources, entry, &mut spent)?;
    Ok(VerifiedExecution {
        program,
        source,
        typed,
        plan,
        entry,
        linear,
        linux,
        runtime,
        abi,
        exports,
    })
}
