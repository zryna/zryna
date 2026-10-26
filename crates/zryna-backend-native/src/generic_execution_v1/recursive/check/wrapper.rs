//! Complete actual scalar wrapper recipe, including lease and counter/output ordering.
use super::{cursor::Cursor, lease};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{FuncRef, Function, Opcode as O, Value, types};
use zryna_abi::{ScalarType, VerifiedScalarExport};
use zryna_diagnostics::Diagnostic;
mod finish;
pub(in crate::generic_execution_v1) mod invocation;
fn guard(c: &mut Cursor<'_>, value: Value) -> Result<(), Diagnostic> {
    let one = c.constant(types::I32, 1)?;
    let invalid =
        c.compare(cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThan, [value, one])?;
    c.trap(invalid)
}
pub(in crate::generic_execution_v1) fn check(
    f: &Function,
    export: VerifiedScalarExport<'_>,
    callee: FuncRef,
    getter: FuncRef,
) -> Result<(), Diagnostic> {
    if f.signature != crate::generic_execution_v1::wrappers::signature(export)
        || f.sized_stack_slots.len() != export.parameters().len() + 2
        || !f.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = f.layout.entry_block().ok_or_else(invariant)?;
    if f.layout.next_block(entry).is_some() {
        return Err(invariant());
    }
    let parameters = f.dfg.block_params(entry);
    if parameters.len() != export.parameters().len() {
        return Err(invariant());
    }
    let mut c = Cursor::new(f, entry);
    let output = c.stack(8, 3)?;
    let mut arguments = Vec::new();
    arguments
        .try_reserve_exact(parameters.len() + 2)
        .map_err(crate::generic_execution_v1::codegen)?;
    arguments.push(output);
    for (&value, ty) in parameters.iter().zip(export.parameters()) {
        if *ty == ScalarType::Bool {
            guard(&mut c, value)?;
        }
        let address = c.stack(8, 3)?;
        let zero = c.constant(types::I64, 0)?;
        c.store(address, 0, zero)?;
        let value =
            if *ty == ScalarType::Bool { c.unary(O::Ireduce, types::I8, value)? } else { value };
        c.store(address, 0, value)?;
        arguments.push(address);
    }
    let ctx = c.stack(32, 3)?;
    let zero = c.constant(types::I64, 0)?;
    for offset in [0, 8, 16, 24] {
        c.store(ctx, offset, zero)?;
    }
    let arena = lease::begin(&mut c, getter, ctx)?;
    arguments.push(ctx);
    let status = c.call(callee, &arguments, types::I32)?;
    lease::completed(&mut c, arena, ctx)?;
    finish::check(&mut c, ctx)?;
    lease::end(&mut c, arena)?;
    c.trap(status)?;
    let result = if export.result() == ScalarType::Bool {
        let value = c.load(types::I8, output, 0)?;
        let value = c.unary(O::Uextend, types::I32, value)?;
        guard(&mut c, value)?;
        value
    } else {
        c.load(types::I32, output, 0)?
    };
    c.returned(&[result])
}
