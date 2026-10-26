//! Closed actual instruction consumer; no expected emitter Function is generated.
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{
    Block, FuncRef, Function, Inst, InstructionData as D, MemFlagsData, Opcode as O, TrapCode,
    Type, Value, condcodes::IntCC,
};
use zryna_diagnostics::Diagnostic;
mod control;
mod memory;
pub(super) struct Cursor<'a> {
    pub f: &'a Function,
    next: Option<Inst>,
    slots: std::collections::BTreeSet<cranelift_codegen::ir::StackSlot>,
}
impl<'a> Cursor<'a> {
    pub fn new(f: &'a Function, block: Block) -> Self {
        Self { f, next: f.layout.first_inst(block), slots: std::collections::BTreeSet::new() }
    }
    pub fn take(&mut self) -> Result<Inst, Diagnostic> {
        let i = self.next.ok_or_else(invariant)?;
        self.next = self.f.layout.next_inst(i);
        Ok(i)
    }
    pub fn value(&self, i: Inst, ty: Type) -> Result<Value, Diagnostic> {
        let values = self.f.dfg.inst_results(i);
        if values.len() != 1 || self.f.dfg.value_type(values[0]) != ty {
            return Err(invariant());
        }
        Ok(values[0])
    }
    pub fn constant(&mut self, ty: Type, expected: i64) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        let D::UnaryImm { opcode: O::Iconst, imm } = self.f.dfg.insts[i] else {
            return Err(invariant());
        };
        if i64::from(imm) != expected {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn load(&mut self, ty: Type, base: Value, displacement: i32) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        let D::Load { opcode: O::Load, arg, offset, .. } = self.f.dfg.insts[i] else {
            return Err(invariant());
        };
        if arg != base
            || i32::from(offset) != displacement
            || self.f.dfg.insts[i].memflags_data(&self.f.dfg) != Some(MemFlagsData::new())
        {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn binary(&mut self, op: O, ty: Type, args: [Value; 2]) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != op || self.f.dfg.inst_args(i) != args {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn compare(&mut self, cc: IntCC, args: [Value; 2]) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        let D::IntCompare { opcode: O::Icmp, cond, args: actual } = self.f.dfg.insts[i] else {
            return Err(invariant());
        };
        if cond != cc || actual != args {
            return Err(invariant());
        }
        self.value(i, cranelift_codegen::ir::types::I8)
    }
    pub fn call(
        &mut self,
        reference: FuncRef,
        args: &[Value],
        ty: Type,
    ) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        if !matches!(self.f.dfg.insts[i],D::Call{func_ref,..} if func_ref==reference)
            || self.f.dfg.inst_args(i) != args
        {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn trap(&mut self, arg: Value) -> Result<(), Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != O::Trapnz
            || self.f.dfg.inst_args(i) != [arg]
            || self.f.dfg.insts[i].trap_code() != Some(TrapCode::unwrap_user(7))
        {
            return Err(invariant());
        }
        Ok(())
    }
    pub fn returned(&mut self, args: &[Value]) -> Result<(), Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != O::Return
            || self.f.dfg.inst_args(i) != args
            || self.next.is_some()
        {
            return Err(invariant());
        }
        Ok(())
    }
}
impl Cursor<'_> {
    pub fn peek(&self) -> Option<Inst> {
        self.next
    }
}
