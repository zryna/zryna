//! Actual bounds predicate, source/index geometry and success-only pointer publication.
use super::super::{body::Frame, invariant};
use super::address;
use address::{args, constant, data};
use cranelift_codegen::ir::{
    Function, Inst, InstructionData as D, Opcode as O, Value, condcodes::IntCC, types,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedIndexedLoan, raw};
use zryna_native_mir::generic_execution_v1::{LoanAddress, TypeCategory};
pub(in crate::generic_execution_v1) fn check(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedIndexedLoan<'_, '_>,
    source: Value,
    status: Value,
    published: Inst,
) -> Result<(), Diagnostic> {
    let base = if let Some(place) = view.base_place() {
        raw::PlaceOrLoan::Place(place.id())
    } else {
        raw::PlaceOrLoan::Loan(view.base_loan().ok_or_else(invariant)?.id())
    };
    address::source(c, f, source, base)?;
    let bounds = bounds(f, status)?;
    let index = args(f, bounds[0], O::Uextend, 1)?[0];
    let D::Load { opcode: O::Load, arg, offset, .. } = data(f, index)? else {
        return Err(invariant());
    };
    if f.dfg.value_type(index) != types::I32 || i32::from(*offset) != 0 {
        return Err(invariant());
    }
    let record = c.f.value(view.index().id()).ok_or_else(invariant)?;
    let raw::ValueType::Stored(ty) = record.ty() else {
        return Err(invariant());
    };
    if c.p.stored_type(ty).ok_or_else(invariant)?.category() != TypeCategory::I32 {
        return Err(invariant());
    }
    // The scalar value's frame record is authoritative even when it has no source Place.
    if args(f, *arg, O::Iadd, 2)?[0] != c.base
        || !constant(f, args(f, *arg, O::Iadd, 2)?[1], i64::from(record.offset()))
    {
        return Err(invariant());
    }
    let stride = geometry(c, f, view, source, bounds[1])?;
    publication(c, f, view, status, published, (source, bounds[0], stride))
}
fn bounds(f: &Function, status: Value) -> Result<[Value; 2], Diagnostic> {
    let choose = args(f, status, O::Select, 3)?;
    if !constant(f, choose[1], 0) || !constant(f, choose[2], 1) {
        return Err(invariant());
    }
    let D::IntCompare { opcode: O::Icmp, cond: IntCC::UnsignedLessThan, args: bounds } =
        data(f, choose[0])?
    else {
        return Err(invariant());
    };
    if f.dfg.value_type(bounds[0]) != types::I64 {
        return Err(invariant());
    }
    let index = args(f, bounds[0], O::Uextend, 1)?[0];
    if f.dfg.value_type(index) != types::I32 {
        return Err(invariant());
    }
    Ok(*bounds)
}

fn geometry(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedIndexedLoan<'_, '_>,
    source: Value,
    length: Value,
) -> Result<u64, Diagnostic> {
    let layout = c.p.stored_type(view.container_type()).ok_or_else(invariant)?;
    match layout.category() {
        TypeCategory::FixedArray => {
            if !constant(
                f,
                length,
                i64::try_from(layout.array_length().ok_or_else(invariant)?)
                    .map_err(|_| invariant())?,
            ) {
                return Err(invariant());
            }
            layout.array_stride().ok_or_else(invariant)
        }
        TypeCategory::Vec => {
            if !load(f, length, source, 8) {
                return Err(invariant());
            }
            Ok(c.p
                .containers()
                .and_then(|p| p.element(view.container_type()))
                .ok_or_else(invariant)?
                .stride())
        }
        _ => Err(invariant()),
    }
}
fn publication(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedIndexedLoan<'_, '_>,
    status: Value,
    published: Inst,
    geometry: (Value, Value, u64),
) -> Result<(), Diagnostic> {
    let (source, index, stride) = geometry;
    let D::Store { opcode: O::Store, args: stored, offset, .. } = f.dfg.insts[published] else {
        return Err(invariant());
    };
    if i32::from(offset) != 0 || f.dfg.value_type(stored[0]) != types::I64 {
        return Err(invariant());
    }
    let LoanAddress::Prepared { slot, offset: 0 } =
        c.f.loan(view.loan().id()).ok_or_else(invariant)?
    else {
        return Err(invariant());
    };
    address::cell(c, f, stored[1], slot)?;
    let pointer = args(f, stored[0], O::Iadd, 2)?;
    let offset = args(f, pointer[1], O::Imul, 2)?;
    if offset[0] != index
        || !constant(f, offset[1], i64::try_from(stride).map_err(|_| invariant())?)
    {
        return Err(invariant());
    }
    let layout = c.p.stored_type(view.container_type()).ok_or_else(invariant)?;
    if layout.category() == TypeCategory::FixedArray {
        if pointer[0] != source {
            return Err(invariant());
        }
    } else if !load(f, pointer[0], source, 0) {
        return Err(invariant());
    }
    let guard = f
        .layout
        .inst_block(f.dfg.value_def(status).inst().ok_or_else(invariant)?)
        .ok_or_else(invariant)?;
    let branch = f.layout.last_inst(guard).ok_or_else(invariant)?;
    let targets =
        f.dfg.insts[branch].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
    if f.dfg.insts[branch].opcode() != O::Brif
        || f.dfg.inst_args(branch).first() != Some(&status)
        || targets.len() != 2
        || Some(targets[1].block(&f.dfg.value_lists)) != f.layout.inst_block(published)
    {
        return Err(invariant());
    }
    if targets[0].block(&f.dfg.value_lists) == targets[1].block(&f.dfg.value_lists) {
        return Err(invariant());
    }
    freshness(c, f, view, source, status, published)?;
    // Existing body::failure consumes the exact pre-child cleanup receipt and returns
    // status through any accounting/drop blocks it emits. This check binds the
    // address publication to its success successor; ELF audit separately checks calls.
    Ok(())
}
fn load(f: &Function, value: Value, address: Value, offset: i32) -> bool {
    matches!(data(f,value),Ok(D::Load { opcode:O::Load,arg,offset:at,.. }) if *arg==address && i32::from(*at)==offset && f.dfg.value_type(value)==types::I64)
}

#[cfg(test)]
mod tests;

// Every mutable descriptor/index observation belongs to this actual Begin preflight.
fn freshness(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedIndexedLoan<'_, '_>,
    source: Value,
    status: Value,
    published: Inst,
) -> Result<(), Diagnostic> {
    let guard = f
        .layout
        .inst_block(f.dfg.value_def(status).inst().ok_or_else(invariant)?)
        .ok_or_else(invariant)?;
    let mut required = [None; 9];
    let mut used = 0;
    let mut current = |value: Value| -> Result<(), Diagnostic> {
        let inst = f.dfg.value_def(value).inst().ok_or_else(invariant)?;
        if f.layout.inst_block(inst) != Some(guard) {
            return Err(invariant());
        }
        *required.get_mut(used).ok_or_else(invariant)? = Some(inst);
        used += 1;
        Ok(())
    };
    current(source)?;
    if let Some(base) = view.base_loan()
        && matches!(c.f.loan(base.id()), Some(LoanAddress::Prepared { .. }))
    {
        let pointer = args(f, source, O::Iadd, 2)?[0];
        current(pointer)?;
        let D::Load { arg, .. } = data(f, pointer)? else { return Err(invariant()) };
        current(*arg)?;
    }
    let bounds = bounds(f, status)?;
    current(bounds[0])?;
    let index = args(f, bounds[0], O::Uextend, 1)?[0];
    current(index)?;
    let D::Load { arg, .. } = data(f, index)? else { return Err(invariant()) };
    current(*arg)?;
    let predicate = args(f, status, O::Select, 3)?[0];
    current(predicate)?;
    if c.p.stored_type(view.container_type()).ok_or_else(invariant)?.category() == TypeCategory::Vec
    {
        current(bounds[1])?;
        let D::Store { args: stored, .. } = f.dfg.insts[published] else { return Err(invariant()) };
        current(args(f, stored[0], O::Iadd, 2)?[0])?;
    }
    // Begin guards split the current block. Across all sites these instruction
    // intervals are disjoint; existing source/emission/cleanup prepay covers visits.
    let mut started = false;
    for inst in f.layout.block_insts(guard) {
        started |= required.contains(&Some(inst));
        if started && matches!(f.dfg.insts[inst].opcode(), O::Call | O::CallIndirect | O::Store) {
            return Err(invariant());
        }
    }
    Ok(())
}
