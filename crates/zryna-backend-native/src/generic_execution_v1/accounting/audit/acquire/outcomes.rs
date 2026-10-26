//! Shared actual String operation code sets and impossible-success predicate proof.
use super::{D, Inst, LogicalOperation as Role, O, Value, invariant};
use cranelift_codegen::ir::{Function, TrapCode, Type, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
mod values;
use values::{binary, code, unary};
pub(super) fn check(
    function: &Function,
    canonical: Inst,
    context: Value,
    operation: Role,
) -> Result<(), Diagnostic> {
    if !matches!(operation, Role::StringFromUtf8Copy | Role::StringClone | Role::StringConcat) {
        return Err(invariant());
    }
    let block = function.layout.inst_block(canonical).ok_or_else(invariant)?;
    let result = *function.dfg.inst_results(canonical).first().ok_or_else(invariant)?;
    let possible = possible(function, canonical, context)?;
    let mut after = false;
    let mut status = false;
    let mut impossible = false;
    for inst in function.layout.block_insts(block) {
        if inst == canonical {
            after = true;
            continue;
        }
        if !after {
            continue;
        }
        let data = &function.dfg.insts[inst];
        if data.opcode() == O::Trapz {
            if status || data.trap_code() != Some(TrapCode::unwrap_user(7)) {
                return Err(invariant());
            }
            let predicate = *function.dfg.inst_args(inst).first().ok_or_else(invariant)?;
            let predicate = if operation == Role::StringFromUtf8Copy {
                let extension = binary(function, predicate, O::Bor, types::I8)?;
                code(function, extension[1], result, 4)?;
                extension[0]
            } else {
                predicate
            };
            let outer = binary(function, predicate, O::Bor, types::I8)?;
            let first = binary(function, outer[0], O::Bor, types::I8)?;
            code(function, first[0], result, 0)?;
            code(function, first[1], result, 1)?;
            code(function, outer[1], result, 2)?;
            status = true;
        }
        if data.opcode() == O::Trapnz {
            if impossible || data.trap_code() != Some(TrapCode::unwrap_user(7)) {
                return Err(invariant());
            }
            let predicate = *function.dfg.inst_args(inst).first().ok_or_else(invariant)?;
            let both = binary(function, predicate, O::Band, types::I8)?;
            code(function, both[0], possible, 0)?;
            code(function, both[1], result, 0)?;
            impossible = true;
        }
    }
    if !status || !impossible {
        return Err(invariant());
    }
    Ok(())
}
fn possible(function: &Function, canonical: Inst, context: Value) -> Result<Value, Diagnostic> {
    let block = function.layout.inst_block(canonical).ok_or_else(invariant)?;
    let mut held = None;
    for inst in function.layout.block_insts(block).take_while(|inst| *inst != canonical) {
        if let D::Store { opcode: O::Store, args, offset, .. } = function.dfg.insts[inst]
            && args[1] == context
            && i32::from(offset) == 24
            && held.replace(args[0]).is_some()
        {
            return Err(invariant());
        }
    }
    let total = held.ok_or_else(invariant)?;
    let sum = binary(function, total, O::Iadd, types::I64)?;
    let debt = unary(function, sum[1], O::Uextend, types::I64)?;
    if function.dfg.value_type(debt) != types::I8 {
        return Err(invariant());
    }
    Ok(debt)
}
