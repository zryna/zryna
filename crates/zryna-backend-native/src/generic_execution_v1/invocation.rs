//! Opaque source-bound invocation objects; ordinary scalar objects retain their exact path.
use crate::{LinuxX8664ObjectTarget, ValidatedNativeObjectArtifact};
use zryna_abi::{Invocation, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) mod plan;
pub(in crate::generic_execution_v1) mod source;
mod status;
pub(super) mod wrapper;
/// Reserved compiler-private host entry of a standalone invocation object.
pub const ENTRY_SYMBOL: &str = "zryna_gexecution_invoke_v1";
/// A separately emitted, audited invocation retaining its exact new source and native MIR.
/// Raw objects, raw requests and older seals cannot construct this artifact.
pub struct ValidatedInvocationArtifact<'m, 'p, 'a> {
    mir: &'m VerifiedProgram<'p, 'a>,
    plan: plan::Plan<'p>,
    object: ValidatedNativeObjectArtifact,
}
impl<'m, 'p, 'a> ValidatedInvocationArtifact<'m, 'p, 'a> {
    /// Exact new source owner of every compiled function and runtime effect.
    #[must_use]
    pub const fn source(&self) -> &'p VerifiedExecution<'a> {
        self.mir.program()
    }
    /// Exact retaining native physical plan, never a reconstructed raw MIR.
    #[must_use]
    pub const fn mir(&self) -> &'m VerifiedProgram<'p, 'a> {
        self.mir
    }
    /// Request internally verified against this exact source owner's scalar ABI.
    #[must_use]
    pub const fn invocation(&self) -> &VerifiedInvocation<'p> {
        &self.plan.request
    }
    /// Independently audited standalone ELF object; ordinary object must not be linked alongside it.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.object.bytes()
    }
    /// Real private host entry, without local-symbol or TLS projections.
    #[must_use]
    pub const fn entry_symbol(&self) -> &str {
        ENTRY_SYMBOL
    }
    /// Total actual native work plus the prepaid invocation additions.
    #[must_use]
    pub const fn codegen_work_units(&self) -> u64 {
        self.plan.work
    }
    /// Rechecks staged bytes against this artifact's same-source invocation inventory.
    /// # Errors
    /// Rejects foreign/mutated sections, symbols, extents, calls and relocations.
    pub fn audit_bytes(&self, bytes: &[u8]) -> Result<(), Diagnostic> {
        if bytes != self.bytes() {
            return Err(super::invariant());
        }
        super::audit::check_invocation(bytes, self.mir, &self.plan)
    }
}
/// Emits a standalone typed invocation with baked scalar arguments and source-status transport.
/// Ordinary source export wrappers and objects are unchanged.
/// # Errors
/// Rejects a foreign/ill-typed request, unchanged work ceiling or any owning object audit failure.
pub fn emit_invocation<'m, 'p, 'a>(
    mir: &'m VerifiedProgram<'p, 'a>,
    request: Invocation,
    _target: LinuxX8664ObjectTarget,
) -> Result<ValidatedInvocationArtifact<'m, 'p, 'a>, Diagnostic> {
    let plan = plan::Plan::derive(mir, request)?;
    let bytes = super::emit(mir, Some(&plan))?;
    super::audit::check_invocation(&bytes, mir, &plan)?;
    Ok(ValidatedInvocationArtifact { mir, plan, object: ValidatedNativeObjectArtifact { bytes } })
}
/// Private closed choice; only a verified invocation plan selects the alternate export.
#[derive(Clone, Copy)]
pub(super) enum Entry {
    Ordinary,
    Invocation(usize),
}
impl Entry {
    pub fn from_plan(plan: Option<&plan::Plan<'_>>) -> Self {
        plan.map_or(Self::Ordinary, |p| Self::Invocation(p.target))
    }
    pub fn wrappers<'s>(
        self,
        p: &'s VerifiedProgram<'_, '_>,
    ) -> impl Iterator<Item = (&'s str, usize)> {
        let ordinary = matches!(self, Self::Ordinary);
        let selected = match self {
            Self::Ordinary => None,
            Self::Invocation(target) => Some((ENTRY_SYMBOL, target)),
        };
        p.program()
            .scalar_abi()
            .exports()
            .filter(move |_| ordinary)
            .map(move |e| {
                (e.native_linux_x86_64_symbol().as_str(), p.program().export_functions()[e.index()])
            })
            .chain(selected)
    }
}
