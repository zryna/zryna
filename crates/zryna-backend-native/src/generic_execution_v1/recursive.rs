//! Loader-prepared traversal storage and closed private iterative Drop protocol.
use super::codegen;
use cranelift_codegen::ir::{AbiParam, Signature, types};
use cranelift_codegen::isa::CallConv;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectModule, ObjectProduct};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;

pub(super) mod audit;
pub(super) mod calls;
pub(super) mod check;
mod definitions;
pub(super) mod emit;
pub(super) mod lifecycle;
pub(super) const TLS_NAME: &str = "zryna_gexecution_traversal_tls";
pub(super) const STUB_NAME: &str = "zryna_gexecution_tls_address";
const ADDRESS: [u8; 17] =
    [0x64, 0x48, 0x8b, 0x04, 0x25, 0, 0, 0, 0, 0x48, 0x8d, 0x80, 0, 0, 0, 0, 0xc3];
pub(super) struct Arena {
    data: DataId,
    address: FuncId,
}
impl Arena {
    pub fn declare(
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
    ) -> Result<Option<Self>, Diagnostic> {
        if !p.requires_recursive_arena() {
            return Ok(None);
        }
        let data = object.declare_data(TLS_NAME, Linkage::Local, true, true).map_err(codegen)?;
        let mut signature = Signature::new(CallConv::SystemV);
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I64));
        let address =
            object.declare_function(STUB_NAME, Linkage::Local, &signature).map_err(codegen)?;
        Ok(Some(Self { data, address }))
    }
    pub fn define(&self, object: &mut ObjectModule) -> Result<(), Diagnostic> {
        let bytes = zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS
            .checked_mul(32)
            .and_then(|n| n.checked_add(16))
            .ok_or_else(super::invariant)?;
        let mut data = DataDescription::new();
        data.define_zeroinit(usize::try_from(bytes).map_err(codegen)?);
        data.set_align(16);
        object.define_data(self.data, &data).map_err(codegen)?;
        object.define_function_bytes(self.address, 1, &ADDRESS, &[]).map_err(codegen)?;
        Ok(())
    }
    pub fn relocate(&self, product: &mut ObjectProduct) -> Result<(), Diagnostic> {
        let function = product.function_symbol(self.address);
        let symbol = product.object.symbol(function);
        let section = symbol.section.id().ok_or_else(super::invariant)?;
        let offset = symbol.value.checked_add(12).ok_or_else(super::invariant)?;
        let target = product.data_symbol(self.data);
        product
            .object
            .add_relocation(
                section,
                object::write::Relocation {
                    offset,
                    symbol: target,
                    addend: 0,
                    flags: object::RelocationFlags::Elf { r_type: object::elf::R_X86_64_TPOFF32 },
                },
            )
            .map_err(codegen)
    }
}

#[cfg(test)]
mod tests;
