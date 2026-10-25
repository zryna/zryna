//! Same-NEW-owner material for the separately selected generic native runtime.
//! These receipts describe bytes; they grant no linking or host execution.

mod identity;
mod source;
#[cfg(test)]
mod tests;

pub(crate) use identity::RuntimeMaterialIdentity;
use zryna_backend_native::ValidatedNativeObjectArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;

/// Compiled-in runtime text retaining the actual new execution and its native MIR.
pub(crate) struct RuntimeSource<'m, 'p, 'a> {
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    program: &'p VerifiedExecution<'a>,
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    mir: &'m VerifiedProgram<'p, 'a>,
    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    bytes: Vec<u8>,
    // A cross-host Build retains the same owner lifetimes without a runnable native image.
    #[cfg(not(any(all(target_os = "linux", target_arch = "x86_64"), test)))]
    _owner: std::marker::PhantomData<(&'p VerifiedExecution<'a>, &'m VerifiedProgram<'p, 'a>)>,
    identity: RuntimeMaterialIdentity,
}

impl<'m, 'p, 'a> RuntimeSource<'m, 'p, 'a> {
    /// Issue only after checking this exact MIR owner and ordinary program object.
    pub(crate) fn issue(
        program: &'p VerifiedExecution<'a>,
        mir: &'m VerifiedProgram<'p, 'a>,
        object: &ValidatedNativeObjectArtifact,
    ) -> Result<Self, Diagnostic> {
        let linear = program.layouts(StorageTarget::Linear32V1);
        let linux = program.layouts(StorageTarget::LinuxX8664V1);
        if !std::ptr::eq(program, mir.program()) || !program.runtime().is_bound_to(linear, linux) {
            return Err(rejected());
        }
        zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), mir)?;
        let bytes = source::render(program)?;
        let identity = RuntimeMaterialIdentity::new(program, mir, object.bytes(), &bytes);
        Ok(Self {
            #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
            program,
            #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
            mir,
            #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
            bytes,
            #[cfg(not(any(all(target_os = "linux", target_arch = "x86_64"), test)))]
            _owner: std::marker::PhantomData,
            identity,
        })
    }

    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn program(&self) -> &'p VerifiedExecution<'a> {
        self.program
    }

    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) const fn mir(&self) -> &'m VerifiedProgram<'p, 'a> {
        self.mir
    }

    #[cfg(any(all(target_os = "linux", target_arch = "x86_64"), test))]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) const fn identity(&self) -> &RuntimeMaterialIdentity {
        &self.identity
    }
}

fn rejected() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C4160",
        None,
        "generic native runtime material differs from its exact new execution owner",
        "retain the same new execution, audited ordinary object, compiled-in template and exact ABI",
    )
}

fn limit() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C4161",
        None,
        "generic native runtime source exceeds the existing bounded material limit",
        "retain the existing object-byte limit and fallible bounded source reservation",
    )
}
