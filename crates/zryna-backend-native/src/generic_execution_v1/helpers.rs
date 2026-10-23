//! Finite per-type helpers; dynamic values use loops and each call owns its progress locals.
use super::{codegen, invariant, runtime};
use cranelift_codegen::{
    Context,
    ir::{AbiParam, FuncRef, Function, InstBuilder, Signature, UserFuncName, Value, types},
    isa::CallConv,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
use zryna_ownership_runtime_abi::LogicalOperation;
mod cloning;
mod dropping;
mod sequence;
pub(super) struct Ids {
    clones: Vec<Option<FuncId>>,
    drops: Vec<Option<FuncId>>,
}
pub(super) struct References {
    clones: Vec<Option<FuncRef>>,
    drops: Vec<Option<FuncRef>>,
}
impl References {
    pub fn clone_value(
        &self,
        p: &VerifiedProgram<'_, '_>,
        b: &mut FunctionBuilder<'_>,
        ty: u32,
        source: Value,
        destination: Value,
    ) -> Result<Value, Diagnostic> {
        if p.containers().and_then(|plan| plan.clone_node(ty)).is_none() {
            return Err(invariant());
        }
        let function = self.clones.get(ty as usize).copied().flatten().ok_or_else(invariant)?;
        let call = b.ins().call(function, &[source, destination]);
        Ok(b.inst_results(call)[0])
    }
    pub fn drop_value(
        &self,
        p: &VerifiedProgram<'_, '_>,
        b: &mut FunctionBuilder<'_>,
        ty: u32,
        address: Value,
    ) -> Result<(), Diagnostic> {
        if p.stored_type(ty).ok_or_else(invariant)?.drop_kind() == 0 {
            return Ok(());
        }
        let function = self.drops.get(ty as usize).copied().flatten().ok_or_else(invariant)?;
        let call = b.ins().call(function, &[address]);
        let status = b.inst_results(call)[0];
        checked(b, status);
        Ok(())
    }
}
fn signature(clone: bool) -> Signature {
    let mut s = Signature::new(CallConv::SystemV);
    s.params.push(AbiParam::new(types::I64));
    if clone {
        s.params.push(AbiParam::new(types::I64));
    }
    s.returns.push(AbiParam::new(types::I32));
    s
}
impl Ids {
    pub fn declare(
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
    ) -> Result<Self, Diagnostic> {
        let mut clones = Vec::new();
        let mut drops = Vec::new();
        if let Some(plan) = p.containers() {
            clones.try_reserve_exact(plan.clone_symbols().len()).map_err(codegen)?;
            drops.try_reserve_exact(plan.drop_symbols().len()).map_err(codegen)?;
            for name in plan.clone_symbols() {
                clones.push(
                    name.map(|name| {
                        object
                            .declare_function(name, Linkage::Local, &signature(true))
                            .map_err(codegen)
                    })
                    .transpose()?,
                );
            }
            for name in plan.drop_symbols() {
                drops.push(
                    name.map(|name| {
                        object
                            .declare_function(name, Linkage::Local, &signature(false))
                            .map_err(codegen)
                    })
                    .transpose()?,
                );
            }
        }
        Ok(Self { clones, drops })
    }
    pub fn references(
        &self,
        object: &mut ObjectModule,
        function: &mut Function,
    ) -> Result<References, Diagnostic> {
        let mut clones = reserve(self.clones.len())?;
        let mut drops = reserve(self.drops.len())?;
        clones.extend(
            self.clones.iter().map(|id| id.map(|id| object.declare_func_in_func(id, function))),
        );
        drops.extend(
            self.drops.iter().map(|id| id.map(|id| object.declare_func_in_func(id, function))),
        );
        Ok(References { clones, drops })
    }
    pub fn define(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        bc: &mut FunctionBuilderContext,
    ) -> Result<(), Diagnostic> {
        for (clone, ids) in [(true, &self.clones), (false, &self.drops)] {
            for (index, id) in ids.iter().enumerate() {
                if let Some(id) = id {
                    let mut context = Context::for_function(Function::with_name_signature(
                        UserFuncName::user(
                            if clone { 8 } else { 9 },
                            u32::try_from(index).map_err(codegen)?,
                        ),
                        signature(clone),
                    ));
                    let references = self.references(object, &mut context.func)?;
                    let mut runtime = reserve(runtime_ids.len())?;
                    runtime.extend(runtime_ids.iter().map(|id| {
                        let r = object.declare_func_in_func(*id, &mut context.func);
                        context.func.dfg.ext_funcs[r].colocated = true;
                        r
                    }));
                    let helper = Helper {
                        p,
                        refs: &references,
                        runtime: &runtime,
                        ty: u32::try_from(index).map_err(codegen)?,
                    };
                    let mut b = FunctionBuilder::new(&mut context.func, bc);
                    let entry = b.create_block();
                    b.append_block_params_for_function_params(entry);
                    b.switch_to_block(entry);
                    let params = b.block_params(entry).to_vec();
                    if clone {
                        cloning::emit(&helper, &mut b, params[0], params[1])?;
                    } else {
                        dropping::emit(&helper, &mut b, params[0])?;
                    }
                    let ok = b.ins().iconst(types::I32, 0);
                    b.ins().return_(&[ok]);
                    b.seal_all_blocks();
                    b.finalize(object.target_config());
                    object.define_function(*id, &mut context).map_err(codegen)?;
                }
            }
        }
        Ok(())
    }
}
pub(super) struct Helper<'c, 'p, 'a> {
    p: &'c VerifiedProgram<'p, 'a>,
    refs: &'c References,
    runtime: &'c [FuncRef],
    ty: u32,
}
impl Helper<'_, '_, '_> {
    fn runtime(
        &self,
        b: &mut FunctionBuilder<'_>,
        op: LogicalOperation,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        runtime::call(self.p, self.runtime, b, op, args)
    }
    fn checked(
        &self,
        b: &mut FunctionBuilder<'_>,
        op: LogicalOperation,
        args: &[Value],
    ) -> Result<(), Diagnostic> {
        let status = self.runtime(b, op, args)?;
        checked(b, status);
        Ok(())
    }
}
fn checked(b: &mut FunctionBuilder<'_>, status: Value) {
    b.ins().trapnz(status, cranelift_codegen::ir::TrapCode::unwrap_user(7));
}
fn propagate(b: &mut FunctionBuilder<'_>, status: Value) {
    let failed = b.create_block();
    let next = b.create_block();
    b.ins().brif(status, failed, &[], next, &[]);
    b.switch_to_block(failed);
    b.ins().return_(&[status]);
    b.switch_to_block(next);
}

fn reserve<T>(count: usize) -> Result<Vec<T>, Diagnostic> {
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(codegen)?;
    Ok(result)
}
