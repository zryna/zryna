//! Actual-SSA substitutions and ordering mutations; no fake effect/source authority.
use super::*;
use cranelift_codegen::ir::{InstBuilder, InstructionData as D, Opcode as O, types};
fn guard(f: &Function, call: Inst) -> cranelift_codegen::ir::Block {
    let admitted = f.layout.inst_block(call).expect("admitted");
    f.layout.prev_block(f.layout.prev_block(admitted).expect("denied")).expect("guard")
}
fn number(f: &mut Function, before: Inst, ty: cranelift_codegen::ir::Type, value: i64) -> Value {
    let mut cursor = FuncCursor::new(f);
    cursor.goto_inst(before);
    cursor.ins().iconst(ty, value)
}
pub(super) fn wrong_size(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let zero = number(f, call, types::I64, 0);
    f.dfg.inst_args_mut(call)[0] = zero;
}
pub(super) fn hold_zero(f: &mut Function, context: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("admitted");
    let inst = f
        .layout
        .block_insts(block)
        .take_while(|i| *i != call)
        .find(|i| {
            matches!(f.dfg.insts[*i],
        D::Store{args,offset,..} if args[1]==context&&i32::from(offset)==24)
        })
        .expect("provisional H");
    let value = f.dfg.inst_args(inst)[0];
    let add = f.dfg.value_def(value).inst().expect("new H");
    let old = f.dfg.inst_args(add)[0];
    let zero = number(f, add, types::I64, 0);
    f.dfg.insts[add] = D::Binary { opcode: O::Iadd, args: [old, zero] };
}
pub(super) fn wrong_count(f: &mut Function, context: Value, _: &[FuncRef], call: Inst) {
    let block = guard(f, call);
    let control = f.dfg.inst_args(call)[0];
    let inst = f
        .layout
        .block_insts(block)
        .find(|i| matches!(f.dfg.insts[*i],D::Load{arg,..} if arg==control))
        .expect("actual control count");
    if let D::Load { arg, .. } = &mut f.dfg.insts[inst] {
        *arg = context;
    }
}
pub(super) fn upgrade_without_alive(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let inst = f
        .layout
        .block_insts(guard(f, call))
        .find(|i| f.dfg.insts[*i].opcode() == O::Band)
        .expect("room and alive");
    let room = f.dfg.inst_args(inst)[0];
    f.dfg.insts[inst] = D::Binary { opcode: O::Band, args: [room, room] };
}
pub(super) fn expired_as_capacity(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("admitted");
    let inst = f
        .layout
        .block_insts(block)
        .skip_while(|i| *i != call)
        .find(|i| {
            matches!(f.dfg.insts[*i],
        D::UnaryImm{opcode:O::Iconst,imm} if i64::from(imm)==5)
        })
        .expect("canonical Expired5");
    f.dfg.insts[inst] = D::UnaryImm { opcode: O::Iconst, imm: 2.into() };
}
pub(super) fn begin_decrements_live(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("begin");
    let inst = f
        .layout
        .block_insts(block)
        .find(|i| f.dfg.insts[*i].opcode() == O::Isub)
        .expect("live unchanged");
    let old = f.dfg.inst_args(inst)[0];
    let one = number(f, inst, types::I64, 1);
    f.dfg.insts[inst] = D::Binary { opcode: O::Isub, args: [old, one] };
}
pub(super) fn finish_reads_strong(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let inst = finish_load(f, call);
    if let D::Load { offset, .. } = &mut f.dfg.insts[inst] {
        *offset = 0.into();
    }
}
fn finish_load(f: &Function, call: Inst) -> Inst {
    let control = f.dfg.inst_args(call)[0];
    let block = f.layout.inst_block(call).expect("finish");
    f.layout
        .block_insts(block)
        .take_while(|i| *i != call)
        .find(|i| {
            matches!(f.dfg.insts[*i],
        D::Load{arg,offset,..} if arg==control&&i32::from(offset)==4)
        })
        .expect("before-call weak capture")
}
pub(super) fn finish_reads_after_free(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let load = finish_load(f, call);
    let loaded = f.dfg.inst_results(load)[0];
    let block = f.layout.inst_block(call).expect("finish");
    let compare = f
        .layout
        .block_insts(block)
        .find(|i| f.dfg.inst_args(*i).contains(&loaded))
        .expect("weak==1");
    let flag = f.dfg.inst_results(compare)[0];
    let extend = f
        .layout
        .block_insts(block)
        .find(|i| f.dfg.inst_args(*i).contains(&flag))
        .expect("live amount");
    let next = f.layout.next_inst(call).expect("status check");
    for inst in [load, compare, extend] {
        f.layout.remove_inst(inst);
        f.layout.insert_inst(inst, next);
    }
}
pub(super) fn missing_out_check(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = f.layout.inst_block(call).expect("weak release");
    let trap = f
        .layout
        .block_insts(block)
        .skip_while(|i| *i != call)
        .find(|i| {
            if f.dfg.insts[*i].opcode() != O::Trapnz {
                return false;
            }
            f.dfg.value_def(f.dfg.inst_args(*i)[0]).inst().is_some_and(|condition| {
                matches!(
                    f.dfg.insts[condition],
                    D::IntCompare { cond: cranelift_codegen::ir::condcodes::IntCC::NotEqual, .. }
                )
            })
        })
        .expect("deallocated out check");
    f.layout.remove_inst(trap);
}
pub(super) fn zero_after_call(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let output = f.dfg.inst_args(call)[1];
    let block = f.layout.inst_block(call).expect("release");
    let store = f
        .layout
        .block_insts(block)
        .take_while(|i| *i != call)
        .find(|i| {
            matches!(f.dfg.insts[*i],
        D::Store{args,..} if args[1]==output)
        })
        .expect("caller zero");
    let next = f.layout.next_inst(call).expect("status check");
    f.layout.remove_inst(store);
    f.layout.insert_inst(store, next);
}
pub(super) fn narrow_zero(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = guard(f, call);
    let output = *f.dfg.inst_args(call).last().expect("runtime out");
    let store = f
        .layout
        .block_insts(block)
        .find(|i| matches!(f.dfg.insts[*i],D::Store{args,..} if args[1]==output))
        .expect("guard output zero");
    let zero = number(f, store, types::I32, 0);
    if let D::Store { args, .. } = &mut f.dfg.insts[store] {
        args[0] = zero;
    }
}
pub(super) fn stale_counter(
    f: &mut Function,
    context: Value,
    _: &[FuncRef],
    call: Inst,
    offset: i32,
) {
    let target = guard(f, call);
    let fresh_inst = f
        .layout
        .block_insts(target)
        .find(|i| {
            matches!(f.dfg.insts[*i],D::Load{arg,offset:actual,..}
        if arg==context&&i32::from(actual)==offset)
        })
        .expect("current guard counter load");
    let old_inst = f
        .layout
        .blocks()
        .take_while(|b| *b != target)
        .flat_map(|b| f.layout.block_insts(b))
        .find(|i| {
            matches!(f.dfg.insts[*i],D::Load{arg,offset:actual,..}
            if arg==context&&i32::from(actual)==offset)
        })
        .expect("dominating prior canonical event counter load");
    let fresh = f.dfg.inst_results(fresh_inst)[0];
    let old = f.dfg.inst_results(old_inst)[0];
    let instructions = f.layout.blocks().flat_map(|b| f.layout.block_insts(b)).collect::<Vec<_>>();
    for inst in instructions {
        for value in f.dfg.inst_args_mut(inst) {
            if *value == fresh {
                *value = old;
            }
        }
    }
}

pub(super) fn outside_guard_count(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let block = guard(f, call);
    let control = f.dfg.inst_args(call)[0];
    let load = f
        .layout
        .block_insts(block)
        .find(|i| matches!(f.dfg.insts[*i],D::Load{arg,..} if arg==control))
        .expect("actual current control count");
    let previous = f.layout.prev_block(block).expect("dominating helper entry");
    let jump = f.layout.last_inst(previous).expect("entry jumps to guard");
    f.layout.remove_inst(load);
    f.layout.insert_inst(load, jump);
}

// Preserve actual I32 width but change its value: u32::MAX-1 is not the refcount bound.
pub(super) fn wrong_maximum(f: &mut Function, _: Value, _: &[FuncRef], call: Inst) {
    let inst = f
        .layout
        .block_insts(guard(f, call))
        .find(|i| {
            matches!(f.dfg.insts[*i],
        D::UnaryImm{opcode:O::Iconst,imm} if i64::from(imm)==i64::from(u32::MAX))
        })
        .expect("actual canonical I32 maximum");
    assert_eq!(f.dfg.value_type(f.dfg.inst_results(inst)[0]), types::I32);
    f.dfg.insts[inst] = D::UnaryImm { opcode: O::Iconst, imm: i64::from(u32::MAX - 1).into() };
}
