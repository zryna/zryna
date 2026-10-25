//! Close the already independently audited funded-release region: no unclassified effects.
use super::{cursor::Cursor, world::World};
use crate::generic_execution_v1::{invariant, runtime};
use cranelift_codegen::ir::{
    FuncRef, Inst, InstructionData as D, MemFlagsData, Opcode as O, Value, types,
};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
use zryna_ownership_runtime_abi::LogicalOperation as Role;
mod closure;
pub(super) fn reference(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    role: Role,
) -> Result<FuncRef, Diagnostic> {
    let index = if role == Role::StringRelease {
        2
    } else {
        runtime::NAMES.len()
            + p.containers()
                .ok_or_else(invariant)?
                .operations()
                .iter()
                .position(|v| *v == role)
                .ok_or_else(invariant)?
    };
    refs.get(index).copied().ok_or_else(invariant)
}
#[derive(Clone, Copy)]
pub(super) struct Call<'a> {
    pub ctx: Value,
    pub function: FuncRef,
    pub args: &'a [Value],
    pub role: Role,
}
pub(super) fn region<'a>(
    w: &mut World<'a>,
    mut before: Cursor<'a>,
    call: Call<'_>,
) -> Result<Cursor<'a>, Diagnostic> {
    let entry = before.jump(&[])?;
    let mut c = w.at(entry, &[])?;
    let mut instructions = Vec::<Inst>::new();
    instructions.try_reserve_exact(128).map_err(crate::generic_execution_v1::codegen)?;
    let mut canonical = None;
    while let Some(inst) = c.peek() {
        if instructions.len() == 128 {
            return Err(invariant());
        }
        c.take()?;
        instructions.push(inst);
        if let D::Call { func_ref, .. } = c.f.dfg.insts[inst] {
            if canonical.is_some()
                || func_ref != call.function
                || c.f.dfg.inst_args(inst) != call.args
            {
                return Err(invariant());
            }
            canonical = Some(inst);
        }
        if canonical.is_some()
            && matches!(c.f.dfg.insts[inst],D::Store{args,offset,..}
            if args[1]==call.ctx && i32::from(offset)==8)
        {
            closure::check(c.f, &instructions, canonical.ok_or_else(invariant)?, call)?;
            return Ok(c);
        }
    }
    Err(invariant())
}
pub(super) fn allowed_load(
    f: &cranelift_codegen::ir::Function,
    inst: Inst,
    call: Call<'_>,
    before: bool,
) -> Result<(), Diagnostic> {
    let D::Load { arg, offset, .. } = f.dfg.insts[inst] else {
        return Err(invariant());
    };
    let offset = i32::from(offset);
    let ty = f.dfg.value_type(f.dfg.inst_results(inst)[0]);
    let valid = if arg == call.ctx {
        before && [0, 8, 16, 24].contains(&offset) && ty == types::I64
    } else if Some(&arg) == call.args.first() {
        before
            && match call.role {
                Role::StringRelease => offset == 8 && ty == types::I64,
                Role::StrongReleaseBegin => offset == 0 && ty == types::I32,
                Role::StrongReleaseFinish => offset == 4 && ty == types::I32,
                Role::WeakRelease => [0, 4].contains(&offset) && ty == types::I32,
                _ => false,
            }
    } else if call.role == Role::VecReleaseStorage && call.args.get(1) == Some(&arg) {
        before && offset == 16 && ty == types::I64
    } else {
        matches!(call.role, Role::StrongReleaseBegin | Role::WeakRelease)
            && call.args.get(1) == Some(&arg)
            && !before
            && offset == 0
            && ty == types::I32
    };
    if !valid || f.dfg.insts[inst].memflags_data(&f.dfg) != Some(MemFlagsData::new()) {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn allowed_store(
    f: &cranelift_codegen::ir::Function,
    inst: Inst,
    call: Call<'_>,
    before: bool,
) -> Result<(), Diagnostic> {
    let D::Store { opcode: O::Store, args, offset, .. } = f.dfg.insts[inst] else {
        return Err(invariant());
    };
    let offset = i32::from(offset);
    let valid = if args[1] == call.ctx {
        f.dfg.value_type(args[0]) == types::I64
            && if before { [16, 24].contains(&offset) } else { offset == 8 }
    } else {
        before
            && matches!(call.role, Role::StrongReleaseBegin | Role::WeakRelease)
            && call.args.get(1) == Some(&args[1])
            && offset == 0
            && f.dfg.value_type(args[0]) == types::I32
    };
    if !valid || f.dfg.insts[inst].memflags_data(&f.dfg) != Some(MemFlagsData::new()) {
        return Err(invariant());
    }
    Ok(())
}
