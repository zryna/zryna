//! Per-invocation frame addresses dominate every verified CFG block.
use super::{codegen, invariant, records};
use cranelift_codegen::{
    Context,
    ir::{FuncRef, InstBuilder, StackSlotData, StackSlotKind, Value, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedParameter, raw,
};
use zryna_native_mir::generic_execution_v1::{FunctionPlan, LoanAddress, Record, VerifiedProgram};

pub(super) struct Frame<'c, 'p, 'a> {
    pub p: &'c VerifiedProgram<'p, 'a>,
    pub f: &'c FunctionPlan<'p, 'a>,
    pub base: Value,
    pub parameters: Vec<Value>,
    pub runtime: &'c [FuncRef],
    pub helpers: &'c super::helpers::References,
    pub callees: &'c BTreeMap<usize, FuncRef>,
}
impl Frame<'_, '_, '_> {
    pub fn record(&self, b: &mut FunctionBuilder<'_>, record: Record) -> Value {
        b.ins().iadd_imm_u(self.base, i64::from(record.offset()))
    }
    pub fn value(&self, b: &mut FunctionBuilder<'_>, id: u32) -> Result<Value, Diagnostic> {
        Ok(self.record(b, self.f.value(id).ok_or_else(invariant)?))
    }
    pub fn place(&self, b: &mut FunctionBuilder<'_>, id: u32) -> Result<Value, Diagnostic> {
        Ok(self.record(b, self.f.place(id).ok_or_else(invariant)?))
    }
    pub fn loan(&self, b: &mut FunctionBuilder<'_>, id: u32) -> Result<Value, Diagnostic> {
        Ok(match self.f.loan(id).ok_or_else(invariant)? {
            LoanAddress::Frame(record) => self.record(b, record),
            LoanAddress::Formal { parameter, offset } => b.ins().iadd_imm_u(
                *self.parameters.get(parameter as usize + 1).ok_or_else(invariant)?,
                i64::from(offset),
            ),
        })
    }
    pub fn source(
        &self,
        b: &mut FunctionBuilder<'_>,
        source: raw::PlaceOrLoan,
    ) -> Result<Value, Diagnostic> {
        match source {
            raw::PlaceOrLoan::Place(id) => self.place(b, id),
            raw::PlaceOrLoan::Loan(id) => self.loan(b, id),
        }
    }
    pub fn size(&self, ty: raw::ValueType) -> Result<u64, Diagnostic> {
        self.p.size(ty).ok_or_else(invariant)
    }
}
pub(super) fn callees(
    p: &VerifiedProgram<'_, '_>,
    index: usize,
) -> Result<BTreeSet<usize>, Diagnostic> {
    let mut result = BTreeSet::new();
    for b in p.functions().get(index).ok_or_else(invariant)?.source().blocks() {
        for op in b.operations() {
            if let Some(target) = op.call_target() {
                result.insert(target.index());
            }
        }
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub(super) struct Calls<'c> {
    pub callees: &'c BTreeMap<usize, FuncRef>,
    pub runtime: &'c [FuncRef],
    pub helpers: &'c super::helpers::References,
}
pub(super) fn build(
    p: &VerifiedProgram<'_, '_>,
    index: usize,
    context: &mut Context,
    bc: &mut FunctionBuilderContext,
    calls: Calls<'_>,
    config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), Diagnostic> {
    let f = p.functions().get(index).ok_or_else(invariant)?;
    let mut b = FunctionBuilder::new(&mut context.func, bc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    let parameters = b.block_params(entry).to_vec();
    let blocks = f.source().blocks().map(|_| b.create_block()).collect::<Vec<_>>();
    let block_values = f
        .source()
        .blocks()
        .map(|block| {
            block
                .value_parameters()
                .map(zryna_ir::generic_v1::execution_v1::VerifiedValue::id)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    b.switch_to_block(entry);
    let slot = b.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        f.frame_bytes(),
        3,
    ));
    let base = b.ins().stack_addr(types::I64, slot, 0);
    records::zero(&mut b, base, u64::from(f.frame_bytes()))?;
    let c = Frame {
        p,
        f,
        base,
        parameters,
        runtime: calls.runtime,
        callees: calls.callees,
        helpers: calls.helpers,
    };
    for (ordinal, parameter) in f.source().parameters().enumerate() {
        if let VerifiedParameter::Value(value) = parameter {
            let destination = c.value(&mut b, value.id())?;
            records::copy(&mut b, c.parameters[ordinal + 1], destination, c.size(value.ty())?)?;
        }
    }
    b.ins().jump(blocks[0], &[]);
    for block in f.source().blocks() {
        b.switch_to_block(blocks[block.id() as usize]);
        for op in block.operations() {
            super::operations::emit(&c, &mut b, op)?;
        }
        super::control::emit(&c, &mut b, block, &blocks, &block_values)?;
    }
    b.seal_all_blocks();
    b.finalize(config);
    Ok(())
}
pub(super) fn cleanup(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    plan: VerifiedCleanupPlan<'_, '_>,
) -> Result<(), Diagnostic> {
    for action in plan.actions() {
        match action {
            VerifiedCleanupAction::EndLoan(_) => {}
            VerifiedCleanupAction::DropPlace(place) => {
                let address = c.place(b, place.id())?;
                records::drop_value(c, b, place.ty(), address)?;
            }
            VerifiedCleanupAction::DropInitializedPrefix { .. } => return Err(invariant()),
        }
    }
    Ok(())
}
pub(super) fn failure(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    plan: Option<VerifiedCleanupPlan<'_, '_>>,
    status: Value,
) -> Result<(), Diagnostic> {
    let failed = b.create_block();
    let success = b.create_block();
    b.ins().brif(status, failed, &[], success, &[]);
    b.switch_to_block(failed);
    cleanup(c, b, plan.ok_or_else(invariant)?)?;
    b.ins().return_(&[status]);
    b.switch_to_block(success);
    Ok(())
}
pub(super) fn checked_offset(offset: u64) -> Result<i32, Diagnostic> {
    i32::try_from(offset).map_err(codegen)
}
