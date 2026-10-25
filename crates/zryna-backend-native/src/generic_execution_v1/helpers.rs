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
mod definitions;
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
        invocation: Value,
    ) -> Result<Value, Diagnostic> {
        if p.containers().and_then(|plan| plan.clone_node(ty)).is_none() {
            return Err(invariant());
        }
        let function = self.clones.get(ty as usize).copied().flatten().ok_or_else(invariant)?;
        let call = b.ins().call(function, &[source, destination, invocation]);
        Ok(b.inst_results(call)[0])
    }
    pub fn drop_value(
        &self,
        p: &VerifiedProgram<'_, '_>,
        b: &mut FunctionBuilder<'_>,
        ty: u32,
        address: Value,
        invocation: Value,
    ) -> Result<(), Diagnostic> {
        if p.stored_type(ty).ok_or_else(invariant)?.drop_kind() == 0 {
            return Ok(());
        }
        let function = self.drops.get(ty as usize).copied().flatten().ok_or_else(invariant)?;
        let call = b.ins().call(function, &[address, invocation]);
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
    s.params.push(AbiParam::new(types::I64));
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
}
pub(super) struct Helper<'c, 'p, 'a> {
    p: &'c VerifiedProgram<'p, 'a>,
    refs: &'c References,
    runtime: &'c [FuncRef],
    ty: u32,
    invocation: Value,
}
impl Helper<'_, '_, '_> {
    fn runtime(
        &self,
        b: &mut FunctionBuilder<'_>,
        op: LogicalOperation,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        if matches!(
            op,
            LogicalOperation::StringClone
                | LogicalOperation::StringConcat
                | LogicalOperation::StringFromUtf8Copy
        ) {
            {
                let effect = self.p.string_leaf(self.ty, true)?;
                if effect.operation() != op {
                    return Err(invariant());
                }
                runtime::string(self.p, self.runtime, b, self.invocation, effect, args)
            }
        } else if op == LogicalOperation::VecAllocate {
            let effect = self.p.vector_leaf(self.ty, true)?;
            Ok(runtime::vector(self.p, self.runtime, b, self.invocation, effect, args)?.status)
        } else if matches!(op, LogicalOperation::StrongClone | LogicalOperation::WeakClone) {
            let effect = self.p.control_clone(self.ty)?;
            if effect.operation() != op {
                return Err(invariant());
            }
            runtime::control(self.p, self.runtime, b, self.invocation, effect, args)
        } else {
            Err(invariant())
        }
    }
    fn checked(
        &self,
        b: &mut FunctionBuilder<'_>,
        op: LogicalOperation,
        args: &[Value],
    ) -> Result<(), Diagnostic> {
        if op == LogicalOperation::StringRelease {
            let _effect = self.p.string_leaf(self.ty, false)?;
            return runtime::string_release(self.p, self.runtime, b, self.invocation, args[0]);
        }
        if op == LogicalOperation::VecReleaseStorage {
            let effect = self.p.vector_leaf(self.ty, false)?;
            return runtime::vector_release(self.p, self.runtime, b, self.invocation, effect, args);
        }
        if matches!(
            op,
            LogicalOperation::StrongReleaseBegin
                | LogicalOperation::StrongReleaseFinish
                | LogicalOperation::WeakRelease
        ) {
            let effect =
                self.p.control_drop(self.ty, op == LogicalOperation::StrongReleaseFinish)?;
            if effect.operation() != op {
                return Err(invariant());
            }
            return runtime::control_release(
                self.p,
                self.runtime,
                b,
                self.invocation,
                effect,
                args,
            );
        }
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

#[cfg(test)]
mod audit_tests;
