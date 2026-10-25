//! Exact actual stores and distinct fixed stack slots.
use super::{Cursor, D, Diagnostic, MemFlagsData, O, Value, invariant};
impl Cursor<'_> {
    pub fn store(
        &mut self,
        base: Value,
        displacement: i32,
        value: Value,
    ) -> Result<(), Diagnostic> {
        let i = self.take()?;
        if !matches!(self.f.dfg.insts[i], D::Store{opcode:O::Store,args,offset,..}
            if args==[value,base] && i32::from(offset)==displacement)
            || self.f.dfg.insts[i].memflags_data(&self.f.dfg) != Some(MemFlagsData::new())
        {
            return Err(invariant());
        }
        Ok(())
    }
    pub fn stack(&mut self, size: u32, align_shift: u8) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        let D::StackAddr { opcode: O::StackAddr, stack_slot, offset } = self.f.dfg.insts[i] else {
            return Err(invariant());
        };
        if !self.slots.insert(stack_slot) {
            return Err(invariant());
        }
        let slot = &self.f.sized_stack_slots[stack_slot];
        if slot.size != size
            || slot.align_shift != align_shift
            || slot.kind != cranelift_codegen::ir::StackSlotKind::ExplicitSlot
            || i32::from(offset) != 0
        {
            return Err(invariant());
        }
        self.value(i, cranelift_codegen::ir::types::I64)
    }
}
