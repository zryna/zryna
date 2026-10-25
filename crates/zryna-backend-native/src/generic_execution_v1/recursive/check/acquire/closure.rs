//! Every acquisition instruction has a classified memory/effect/dataflow role.
use super::{Call, D, Inst, O, Role, Value, invariant};
use cranelift_codegen::ir::{Block, Function, MemFlagsData, TrapCode, types};
use std::collections::BTreeSet;
use zryna_diagnostics::Diagnostic;
pub(super) fn check(
    function: &Function,
    instructions: &[Inst],
    canonical: Inst,
    condition: Value,
    result: Value,
    call: Call<'_>,
    guard: Block,
) -> Result<(), Diagnostic> {
    let mut roots = BTreeSet::from([condition, result]);
    let mut before = true;
    let mut traps = 0;
    for &inst in instructions {
        let data = &function.dfg.insts[inst];
        let in_guard = function.layout.inst_block(inst) == Some(guard);
        match data {
            D::Call { .. } if inst == canonical => {
                before = false;
                roots.extend(function.dfg.inst_args(inst));
            }
            D::Load { opcode: O::Load, arg, offset, .. } => {
                let width = function.dfg.value_type(function.dfg.inst_results(inst)[0]);
                let offset = i32::from(*offset);
                let valid = in_guard
                    && before
                    && if *arg == call.context {
                        [0, 8, 16, 24].contains(&offset) && width == types::I64
                    } else {
                        Some(arg) == call.args.first()
                            && match call.role {
                                Role::StringClone => offset == 8 && width == types::I64,
                                Role::StrongClone => offset == 0 && width == types::I32,
                                Role::WeakClone => offset == 4 && width == types::I32,
                                _ => false,
                            }
                    };
                if !valid || data.memflags_data(&function.dfg) != Some(MemFlagsData::new()) {
                    return Err(invariant());
                }
            }
            D::Store { opcode: O::Store, args, offset, .. } => {
                let offset = i32::from(*offset);
                let valid = if args[1] == call.context {
                    !in_guard && [0, 8, 16, 24].contains(&offset)
                } else {
                    in_guard
                        && before
                        && matches!(call.role, Role::StringClone | Role::VecAllocate)
                        && call.args.last() == Some(&args[1])
                        && [0, 8, 16].contains(&offset)
                };
                if !valid
                    || function.dfg.value_type(args[0]) != types::I64
                    || data.memflags_data(&function.dfg) != Some(MemFlagsData::new())
                {
                    return Err(invariant());
                }
                roots.extend(function.dfg.inst_args(inst));
            }
            _ if matches!(data.opcode(), O::Trapnz | O::Trapz) => {
                if data.trap_code() != Some(TrapCode::unwrap_user(7)) {
                    return Err(invariant());
                }
                roots.extend(function.dfg.inst_args(inst));
                traps += 1;
            }
            _ if matches!(
                data.opcode(),
                O::Iconst | O::Icmp | O::Bor | O::Band | O::Iadd | O::Isub | O::Uextend | O::Select
            ) => {}
            _ => return Err(invariant()),
        }
    }
    let expected = if matches!(call.role, Role::StrongClone | Role::WeakClone) { 5 } else { 4 };
    if before || traps != expected {
        return Err(invariant());
    }
    for &inst in instructions.iter().rev() {
        let results = function.dfg.inst_results(inst);
        if results.is_empty() {
            continue;
        }
        if !results.iter().any(|value| roots.contains(value)) {
            return Err(invariant());
        }
        roots.extend(function.dfg.inst_args(inst));
    }
    Ok(())
}
