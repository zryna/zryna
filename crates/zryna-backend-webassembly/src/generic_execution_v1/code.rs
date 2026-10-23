//! Bounded canonical code recipes over retained new-domain views and checked addresses.
use super::{
    Diagnostic, budget,
    bytes::Bytes,
    invariant,
    model::{self, Model},
};
use wasm_encoder::{Instruction as I, MemArg};
mod aggregates;
mod containers;
mod observer;
mod operations;
mod runtime;
mod source;
mod transfers;
mod wrappers;
pub(super) const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
pub(super) const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };
pub(super) struct Body<'b> {
    ledger: &'b std::cell::Cell<usize>,
    pub(super) bytes: Bytes,
}
impl<'b> Body<'b> {
    pub(super) fn new(m: &'b Model<'_, '_, '_>, locals: u32) -> Result<Self, Diagnostic> {
        if locals > 127 {
            return Err(invariant());
        }
        let size = if locals == 0 { 1 } else { 3 };
        let next = m
            .writer
            .get()
            .checked_add(size)
            .filter(|n| *n <= super::MAX_BYTES)
            .ok_or_else(budget)?;
        m.writer.set(next);
        let mut bytes = Bytes::new();
        if locals == 0 {
            bytes.extend(&[0])?;
        } else {
            bytes.extend(&[1, crate::generic_execution_v1::checked_u8(locals)?, 0x7f])?;
        }
        Ok(Self { bytes, ledger: &m.writer })
    }
    pub(super) fn op(&mut self, i: &I<'static>) -> Result<(), Diagnostic> {
        let mut encoded = Bytes::new();
        encoded.op(i)?;
        let next = self
            .ledger
            .get()
            .checked_add(encoded.bytes.len())
            .filter(|n| *n <= super::MAX_BYTES)
            .ok_or_else(budget)?;
        self.ledger.set(next);
        self.bytes.extend(&encoded.bytes)
    }
    pub(super) fn ops(
        &mut self,
        ops: impl IntoIterator<Item = I<'static>>,
    ) -> Result<(), Diagnostic> {
        for op in ops {
            self.op(&op)?;
        }
        Ok(())
    }
    pub(super) fn addr(&mut self, local: u32, offset: u32) -> Result<(), Diagnostic> {
        self.ops([
            I::LocalGet(local),
            I::I32Const(crate::generic_execution_v1::checked_u32(offset)?.cast_signed()),
            I::I32Add,
        ])
    }
    pub(super) fn load(&mut self, local: u32, offset: u32) -> Result<(), Diagnostic> {
        if offset <= 16 {
            return self.ops([
                I::LocalGet(local),
                I::I32Load(MemArg { offset: u64::from(offset), ..WORD }),
            ]);
        }
        // Frame geometry has already bounded this private address within owned memory.
        // Keep the closed memory-op grammar unchanged for larger source-frame offsets.
        self.addr(local, offset)?;
        self.op(&I::I32Load(WORD))
    }
    pub(super) fn finish(mut self) -> Result<Bytes, Diagnostic> {
        self.op(&I::End)?;
        Ok(self.bytes)
    }
}
pub(super) struct FunctionCode {
    pub(super) parameters: u32,
    pub(super) results: u32,
    pub(super) locals: u32,
    pub(super) result64: bool,
    pub(super) body: Bytes,
}
impl FunctionCode {
    fn new(parameters: u32, results: u32, locals: u32, body: Body<'_>) -> Result<Self, Diagnostic> {
        Ok(Self { parameters, results, locals, result64: false, body: body.finish()? })
    }
}
pub(super) fn build(model: &Model<'_, '_, '_>) -> Result<Vec<FunctionCode>, Diagnostic> {
    let count = model.total_functions() as usize;
    model.take(count.checked_mul(size_of::<FunctionCode>()).ok_or_else(budget)?)?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| budget())?;
    runtime::append(model, &mut result)?;
    for index in 0..model.types.len() {
        result.push(aggregates::clone_helper(
            model,
            crate::generic_execution_v1::checked_u32(index)?,
        )?);
    }
    for index in 0..model.types.len() {
        result.push(aggregates::drop_helper(
            model,
            crate::generic_execution_v1::checked_u32(index)?,
        )?);
    }
    for index in 0..model.functions.len() {
        result.push(source::function(model, index)?);
    }
    for &index in model.program.export_functions() {
        result.push(wrappers::wrapper(model, index)?);
    }
    if model.observed() {
        result.push(observer::getter(model)?);
        result.push(observer::recorder(model)?);
    }
    containers::append(model, &mut result)?;
    if result.len() != count {
        return Err(invariant());
    }
    Ok(result)
}
pub(super) fn trace(
    model: &Model<'_, '_, '_>,
    body: &mut Body<'_>,
    fields: [Vec<I<'static>>; 6],
) -> Result<(), Diagnostic> {
    if model.observed() {
        for field in fields {
            body.ops(field)?;
        }
        body.op(&I::Call(model.recorder))?;
    }
    Ok(())
}
pub(super) fn canonical_failure(
    body: &mut Body<'_>,
    status: i32,
    runtime_status: i32,
) -> Result<(), Diagnostic> {
    body.ops([I::I32Const(status), I::GlobalSet(1), I::I32Const(runtime_status), I::Return])
}

#[cfg(test)]
mod tests;
