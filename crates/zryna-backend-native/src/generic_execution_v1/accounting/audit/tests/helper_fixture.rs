//! Helpers use actual source operation/frontier types and the real same-seal emitter.
use super::*;
use crate::generic_execution_v1::{helpers, runtime};
use cranelift_module::FuncId;
use cranelift_object::ObjectModule;
pub(super) struct Graph {
    pub function: Function,
    pub runtime: Vec<FuncRef>,
    pub invocation: Value,
    pub call: Inst,
}
pub(super) fn build(
    p: &VerifiedProgram<'_, '_>,
    object: &mut ObjectModule,
    helpers: &helpers::Ids,
    runtime_ids: &[FuncId],
    operation: Op,
) -> Option<Graph> {
    let clone = matches!(operation, Op::StrongClone | Op::WeakClone);
    let shared =
        matches!(operation, Op::StrongClone | Op::StrongReleaseBegin | Op::StrongReleaseFinish);
    if !matches!(
        operation,
        Op::VecReleaseStorage
            | Op::StrongClone
            | Op::WeakClone
            | Op::StrongReleaseBegin
            | Op::StrongReleaseFinish
            | Op::WeakRelease
    ) {
        return None;
    }
    let wanted = if operation == Op::VecReleaseStorage {
        zryna_native_mir::generic_execution_v1::TypeCategory::Vec
    } else if shared {
        zryna_native_mir::generic_execution_v1::TypeCategory::Shared
    } else {
        zryna_native_mir::generic_execution_v1::TypeCategory::Weak
    };
    let ty = p
        .program()
        .operations()
        .find_map(|op| {
            if clone {
                op.clone_frontier()
                    .filter(|v| p.stored_type(v.ty()).is_some_and(|t| t.category() == wanted))
                    .map(zryna_ir::generic_v1::execution_v1::VerifiedClone::ty)
            } else if operation == Op::VecReleaseStorage {
                op.vec_construct().map(zryna_ir::generic_v1::execution_v1::VerifiedVecConstruct::ty)
            } else if shared {
                op.shared_construct()
                    .map(zryna_ir::generic_v1::execution_v1::VerifiedSharedConstruct::ty)
            } else {
                op.weak_downgrade()
                    .map(zryna_ir::generic_v1::execution_v1::VerifiedWeakDowngrade::ty)
            }
        })
        .expect("genuine source operation or Clone frontier type");
    let (function, runtime, invocation) = if clone {
        helpers.audit_clone_body(object, p, runtime_ids, ty)
    } else {
        helpers.audit_drop_body(object, p, runtime_ids, ty)
    };
    let index = runtime::NAMES.len()
        + p.containers()
            .expect("actual plan")
            .operations()
            .iter()
            .position(|op| *op == operation)
            .expect("actual runtime role");
    let call=function.layout.blocks().flat_map(|b|function.layout.block_insts(b))
        .find(|inst|matches!(function.dfg.insts[*inst],cranelift_codegen::ir::InstructionData::Call{func_ref,..}
            if func_ref==runtime[index])).expect("real generated canonical call");
    Some(Graph { function, runtime, invocation, call })
}
