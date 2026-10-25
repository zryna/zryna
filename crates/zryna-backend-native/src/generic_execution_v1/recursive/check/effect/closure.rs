//! Every fixed release instruction is effect/dataflow classified, not counted as a body proof.
use super::{Call, allowed_load, allowed_store, invariant};
use cranelift_codegen::ir::{
    Function, Inst, InstructionData as D, Opcode as O, TrapCode, Value, types,
};
use std::collections::BTreeSet;
use zryna_diagnostics::Diagnostic;
use zryna_ownership_runtime_abi::LogicalOperation as Role;
pub(super) fn check(
    f: &Function,
    instructions: &[Inst],
    canonical: Inst,
    call: Call<'_>,
) -> Result<(), Diagnostic> {
    let exact = match call.role {
        Role::StringRelease | Role::VecReleaseStorage => 38,
        Role::StrongReleaseBegin => 45,
        Role::StrongReleaseFinish => 40,
        Role::WeakRelease => 50,
        _ => return Err(invariant()),
    };
    if instructions.len() != exact {
        return Err(invariant());
    }
    let mut before = true;
    let mut traps = 0;
    let mut needed = BTreeSet::<Value>::new();
    for &inst in instructions {
        let data = &f.dfg.insts[inst];
        match data {
            D::Call { .. } if inst == canonical => {
                before = false;
                needed.extend(f.dfg.inst_args(inst));
            }
            D::Load { .. } => allowed_load(f, inst, call, before)?,
            D::Store { .. } => {
                allowed_store(f, inst, call, before)?;
                needed.extend(f.dfg.inst_args(inst));
            }
            _ if data.opcode() == O::Trapnz => {
                if data.trap_code() != Some(TrapCode::unwrap_user(7)) {
                    return Err(invariant());
                }
                traps += 1;
                needed.extend(f.dfg.inst_args(inst));
            }
            _ if matches!(
                data.opcode(),
                O::Iconst | O::Icmp | O::Bor | O::Band | O::Iadd | O::Isub | O::Uextend
            ) => {}
            _ => return Err(invariant()),
        }
    }
    let flags = matches!(call.role, Role::StrongReleaseBegin | Role::WeakRelease);
    if traps != if flags { 5 } else { 4 } || before {
        return Err(invariant());
    }
    for &inst in instructions.iter().rev() {
        let data = &f.dfg.insts[inst];
        let outputs = f.dfg.inst_results(inst);
        if outputs.is_empty() {
            continue;
        }
        if outputs.iter().any(|v| needed.contains(v)) {
            needed.extend(f.dfg.inst_args(inst));
        } else {
            // Existing facts::capture emits one leading I64 zero even for Finish/Weak.
            // It is the ONLY permitted unused instruction; arbitrary dead code is rejected.
            let allowed = Some(&inst) == instructions.first()
                && matches!(call.role, Role::StrongReleaseFinish | Role::WeakRelease)
                && matches!(data,D::UnaryImm{opcode:O::Iconst,imm} if i64::from(*imm)==0)
                && outputs.len() == 1
                && f.dfg.value_type(outputs[0]) == types::I64;
            if !allowed {
                return Err(invariant());
            }
        }
    }
    Ok(())
}
