//! Actual Vec graph mutations use no production fault switches or fabricated authority.
use super::*;
use cranelift_codegen::ir::{InstBuilder, InstructionData as D, Opcode as O, types};
fn zero_before(f: &mut Function, inst: Inst) -> Value {
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(inst);
    cursor.ins().iconst(types::I64, 0)
}
fn guard(f: &Function, call: Inst) -> cranelift_codegen::ir::Block {
    let admitted = f.layout.inst_block(call).expect("admitted actual call");
    let denied = f.layout.prev_block(admitted).expect("denied");
    f.layout.prev_block(denied).expect("preflight")
}
pub(super) fn wrong_request(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let index = if f.dfg.inst_args(call).len() == 3 { 1 } else { 2 };
    let zero = zero_before(f, call);
    f.dfg.inst_args_mut(call)[index] = zero;
}
pub(super) fn wrong_doubling(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = guard(f, call);
    let doubled = f
        .layout
        .block_insts(block)
        .find(|inst| {
            f.dfg.insts[*inst].opcode() == O::Iadd
                && f.dfg.inst_args(*inst).len() == 2
                && f.dfg.inst_args(*inst)[0] == f.dfg.inst_args(*inst)[1]
        })
        .expect("actual canonical capacity doubling");
    let left = f.dfg.inst_args(doubled)[0];
    let zero = zero_before(f, doubled);
    f.dfg.insts[doubled] = D::Binary { opcode: O::Iadd, args: [left, zero] };
}
pub(super) fn retire_zero(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("admitted");
    let sub = f
        .layout
        .block_insts(block)
        .find(|i| f.dfg.insts[*i].opcode() == O::Isub)
        .expect("same-event old-block retirement");
    let left = f.dfg.inst_args(sub)[0];
    let zero = zero_before(f, sub);
    f.dfg.insts[sub] = D::Binary { opcode: O::Isub, args: [left, zero] };
}
pub(super) fn missing_allowed_status(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    remove_trap(f, call, O::Trapz);
}
pub(super) fn missing_impossible_status(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    remove_trap(f, call, O::Trapnz);
}
fn remove_trap(f: &mut Function, call: Inst, opcode: O) {
    let block = f.layout.inst_block(call).expect("admitted");
    let inst = f
        .layout
        .block_insts(block)
        .skip_while(|i| *i != call)
        .find(|i| f.dfg.insts[*i].opcode() == opcode)
        .expect("actual status trap");
    f.layout.remove_inst(inst);
}
pub(super) fn release_length(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let address = f.dfg.inst_args(call)[1];
    let block = f.layout.inst_block(call).expect("release");
    let inst = f
        .layout
        .block_insts(block)
        .find(|inst| {
            matches!(f.dfg.insts[*inst],
        D::Load { arg, offset, .. } if arg == address && i32::from(offset) == 16)
        })
        .expect("actual capacity-backed live amount");
    if let D::Load { offset, .. } = &mut f.dfg.insts[inst] {
        *offset = 8.into();
    }
}
