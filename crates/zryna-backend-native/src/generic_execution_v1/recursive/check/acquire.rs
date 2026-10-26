//! Close real fresh-preflight/denied/admitted/merge blocks around independent accounting proofs.
use super::{cursor::Cursor, world::World};
use crate::generic_execution_v1::{invariant, runtime};
use cranelift_codegen::ir::{FuncRef, Inst, InstructionData as D, Opcode as O, Value, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
use zryna_ownership_runtime_abi::LogicalOperation as Role;
mod closure;
#[derive(Clone, Copy)]
pub(super) struct Call<'a> {
    pub context: Value,
    pub function: FuncRef,
    pub args: &'a [Value],
    pub role: Role,
}
pub(super) fn reference(
    program: &VerifiedProgram<'_, '_>,
    functions: &[FuncRef],
    role: Role,
) -> Result<FuncRef, Diagnostic> {
    let index = if role == Role::StringClone {
        1
    } else {
        runtime::NAMES.len()
            + program
                .containers()
                .ok_or_else(invariant)?
                .operations()
                .iter()
                .position(|entry| *entry == role)
                .ok_or_else(invariant)?
    };
    functions.get(index).copied().ok_or_else(invariant)
}
pub(super) fn region<'f>(
    world: &mut World<'f>,
    mut before: Cursor<'f>,
    call: Call<'_>,
) -> Result<(Cursor<'f>, Value), Diagnostic> {
    let entry = before.jump(&[])?;
    let mut guard = world.at(entry, &[])?;
    let mut instructions = Vec::<Inst>::new();
    instructions.try_reserve_exact(96).map_err(crate::generic_execution_v1::codegen)?;
    while let Some(inst) = guard.peek() {
        if guard.f.dfg.insts[inst].opcode() == O::Brif {
            break;
        }
        if instructions.len() == 96 {
            return Err(invariant());
        }
        guard.take()?;
        instructions.push(inst);
    }
    let branch = guard.peek().ok_or_else(invariant)?;
    let condition = *guard.f.dfg.inst_args(branch).first().ok_or_else(invariant)?;
    let (denied, admitted) = guard.branch(condition)?;
    let expected =
        if call.role == Role::StringClone || call.role == Role::VecAllocate { 45 } else { 44 };
    if instructions.len() != expected {
        return Err(invariant());
    }
    let mut denied = world.at(denied, &[])?;
    let capacity = denied.constant(types::I32, 2)?;
    let merge = denied.jump(&[capacity])?;
    let mut admitted = world.at(admitted, &[])?;
    let mut canonical = None;
    let mut admitted_count = 0;
    while let Some(inst) = admitted.peek() {
        if admitted.f.dfg.insts[inst].opcode() == O::Jump {
            break;
        }
        if admitted_count == 32 {
            return Err(invariant());
        }
        admitted.take()?;
        instructions.push(inst);
        admitted_count += 1;
        if let D::Call { func_ref, .. } = admitted.f.dfg.insts[inst] {
            if canonical.is_some()
                || func_ref != call.function
                || admitted.f.dfg.inst_args(inst) != call.args
            {
                return Err(invariant());
            }
            canonical = Some(inst);
        }
    }
    let canonical = canonical.ok_or_else(invariant)?;
    let result = admitted.f.dfg.inst_results(canonical);
    if result.len() != 1 || admitted.f.dfg.value_type(result[0]) != types::I32 {
        return Err(invariant());
    }
    let result = result[0];
    let expected = match call.role {
        Role::StringClone => 23,
        Role::VecAllocate => 24,
        Role::StrongClone | Role::WeakClone => 20,
        _ => return Err(invariant()),
    };
    if admitted_count != expected || admitted.jump(&[result])? != merge {
        return Err(invariant());
    }
    closure::check(world.f, &instructions, canonical, condition, result, call, entry)?;
    let reader = world.at(merge, &[types::I32])?;
    let status = world.f.dfg.block_params(merge)[0];
    Ok((reader, status))
}
