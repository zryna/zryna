//! Change decoded SSA roles independently; no production fault switches.
use super::*;
use cranelift_codegen::ir::{
    InstBuilder, InstructionData as D, MemFlagsData, Opcode as O, condcodes::IntCC, types,
};
fn zero_before(function: &mut Function, inst: Inst) -> Value {
    let mut cursor = FuncCursor::new(function);
    cursor.goto_inst(inst);
    cursor.ins().iconst(types::I64, 0)
}
fn guard(function: &Function, call: Inst) -> cranelift_codegen::ir::Block {
    let admitted = function.layout.inst_block(call).expect("admitted block");
    let denied = function.layout.prev_block(admitted).expect("denied block");
    function.layout.prev_block(denied).expect("preflight block")
}
pub(super) fn wrong_length(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = guard(f, call);
    let comparisons = f
        .layout
        .block_insts(block)
        .filter(|inst| {
            matches!(
                f.dfg.insts[*inst],
                D::IntCompare { cond: IntCC::UnsignedLessThanOrEqual | IntCC::NotEqual, .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(comparisons.len(), 2, "possible/nonempty only in isolated preflight");
    for inst in comparisons {
        let zero = zero_before(f, inst);
        let D::IntCompare { args, .. } = &mut f.dfg.insts[inst] else { unreachable!() };
        args[0] = zero;
    }
}
pub(super) fn reset_after_call(f: &mut Function, context: Value, _: &[FuncRef], call: Inst) {
    let next = f.layout.next_inst(call).expect("post-call bookkeeping");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(next);
    let zero = cursor.ins().iconst(types::I64, 0);
    cursor.ins().store(MemFlagsData::new(), zero, context, 0);
}
pub(super) fn denied_store(f: &mut Function, context: Value, _: &[FuncRef], call: Inst) {
    let admitted = f.layout.inst_block(call).expect("admitted");
    let denied = f.layout.prev_block(admitted).expect("denied");
    let jump = f.layout.last_inst(denied).expect("denied jump");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(jump);
    let zero = cursor.ins().iconst(types::I64, 0);
    cursor.ins().store(MemFlagsData::new(), zero, context, 0);
}
pub(super) fn stray_store(f: &mut Function, context: Value, _: &[FuncRef], _: Inst) {
    let entry = f.layout.entry_block().expect("entry");
    let inst = f.layout.first_inst(entry).expect("body entry");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(inst);
    let zero = cursor.ins().iconst(types::I64, 0);
    cursor.ins().store(MemFlagsData::new(), zero, context, 0);
}
pub(super) fn alias_context(f: &mut Function, context: Value, _: &[FuncRef], call: Inst) {
    let jump = f.layout.last_inst(guard(f, call)).expect("preflight branch");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(jump);
    let zero = cursor.ins().iconst(types::I64, 0);
    let alias = cursor.ins().iadd(context, zero);
    cursor.ins().store(MemFlagsData::new(), zero, alias, 0);
}
pub(super) fn poison_output(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let output = *f.dfg.inst_args(call).last().expect("real runtime out");
    let branch = f.layout.last_inst(guard(f, call)).expect("guard branch");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(branch);
    let poison = cursor.ins().iconst(types::I64, 1);
    cursor.ins().store(MemFlagsData::new(), poison, output, 0);
}
pub(super) fn missing_guard(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let branch = f.layout.last_inst(guard(f, call)).expect("guard branch");
    let condition = f.dfg.inst_args(branch)[0];
    let inst = f.dfg.value_def(condition).inst().expect("guard expression");
    f.dfg.insts[inst] = D::UnaryImm { opcode: O::Iconst, imm: 0.into() };
}
pub(super) fn release_subtract_zero(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("release block");
    let sub = f
        .layout
        .block_insts(block)
        .find(|inst| f.dfg.insts[*inst].opcode() == O::Isub)
        .expect("actual release live subtraction");
    let left = f.dfg.inst_args(sub)[0];
    let zero = zero_before(f, sub);
    f.dfg.insts[sub] = D::Binary { opcode: O::Isub, args: [left, zero] };
}
pub(super) fn wrong_callee_context(f: &mut Function, context: Value, runtime: &[FuncRef], _: Inst) {
    let call = f.layout.blocks().flat_map(|block| f.layout.block_insts(block))
        .find(|inst| matches!(f.dfg.insts[*inst], D::Call{func_ref,..} if !runtime.contains(&func_ref)))
        .expect("real source callee");
    assert_eq!(f.dfg.inst_args(call).last(), Some(&context));
    let zero = zero_before(f, call);
    let args = f.dfg.inst_args_mut(call);
    *args.last_mut().expect("actual context slot") = zero;
}
