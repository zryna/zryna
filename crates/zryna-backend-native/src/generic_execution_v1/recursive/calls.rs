//! Exact private signatures and references; all calls retain the original context last.
use crate::generic_execution_v1::codegen;
use cranelift_codegen::{
    ir::{AbiParam, FuncRef, Function, Signature, types},
    isa::CallConv,
};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
#[derive(Clone, Copy, Debug)]
pub(in crate::generic_execution_v1) enum Kind {
    Arena,
    Find,
    Drop,
    Release,
    Init,
    Clone,
}
pub(in crate::generic_execution_v1) const KINDS: [Kind; 6] =
    [Kind::Arena, Kind::Find, Kind::Drop, Kind::Release, Kind::Init, Kind::Clone];
pub(in crate::generic_execution_v1) fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Arena => "zryna_gexecution_iter_arena",
        Kind::Find => "zryna_gexecution_iter_find",
        Kind::Drop => "zryna_gexecution_iter_drop",
        Kind::Release => "zryna_gexecution_iter_vec_release",
        Kind::Init => "zryna_gexecution_iter_init",
        Kind::Clone => "zryna_gexecution_iter_clone",
    }
}
pub(in crate::generic_execution_v1) fn signature(kind: Kind) -> Signature {
    let mut s = Signature::new(CallConv::SystemV);
    let parameters: &[cranelift_codegen::ir::Type] = match kind {
        Kind::Arena => &[types::I64],
        Kind::Find => &[types::I32, types::I64, types::I32, types::I32, types::I64],
        Kind::Drop => &[types::I32, types::I64, types::I32, types::I64],
        Kind::Release => &[types::I32, types::I64, types::I64],
        Kind::Init | Kind::Clone => &[types::I32, types::I64, types::I64, types::I64],
    };
    s.params.extend(parameters.iter().copied().map(AbiParam::new));
    s.returns.push(AbiParam::new(if matches!(kind, Kind::Arena | Kind::Find) {
        types::I64
    } else {
        types::I32
    }));
    s
}
pub(in crate::generic_execution_v1) struct Ids {
    pub getter: FuncId,
    pub functions: [FuncId; 6],
}
#[derive(Clone, Copy)]
pub(in crate::generic_execution_v1) struct Refs {
    pub getter: FuncRef,
    pub arena: FuncRef,
    pub find: FuncRef,
    pub drop: FuncRef,
    pub release: FuncRef,
    pub init: FuncRef,
    pub clone: FuncRef,
}
impl Ids {
    pub fn declare(
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
    ) -> Result<Option<Self>, Diagnostic> {
        if !p.requires_recursive_arena() {
            return Ok(None);
        }
        let mut getter_signature = Signature::new(CallConv::SystemV);
        getter_signature.params.push(AbiParam::new(types::I64));
        getter_signature.returns.push(AbiParam::new(types::I64));
        let getter = object
            .declare_function(super::STUB_NAME, Linkage::Local, &getter_signature)
            .map_err(codegen)?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(6).map_err(codegen)?;
        for kind in KINDS {
            ids.push(
                object
                    .declare_function(name(kind), Linkage::Local, &signature(kind))
                    .map_err(codegen)?,
            );
        }
        Ok(Some(Self { getter, functions: ids.try_into().map_err(|_| super::super::invariant())? }))
    }
    pub fn references(&self, object: &mut ObjectModule, f: &mut Function) -> Refs {
        Refs {
            getter: object.declare_func_in_func(self.getter, f),
            arena: object.declare_func_in_func(self.functions[0], f),
            find: object.declare_func_in_func(self.functions[1], f),
            drop: object.declare_func_in_func(self.functions[2], f),
            release: object.declare_func_in_func(self.functions[3], f),
            init: object.declare_func_in_func(self.functions[4], f),
            clone: object.declare_func_in_func(self.functions[5], f),
        }
    }
}
