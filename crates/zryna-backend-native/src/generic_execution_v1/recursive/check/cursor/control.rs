//! Exact actual control-flow and unary/selection operations.
use super::{Block, Cursor, Diagnostic, O, TrapCode, Type, Value, invariant};
impl Cursor<'_> {
    pub fn unary(&mut self, op: O, ty: Type, arg: Value) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != op || self.f.dfg.inst_args(i) != [arg] {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn ternary(&mut self, op: O, ty: Type, args: [Value; 3]) -> Result<Value, Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != op || self.f.dfg.inst_args(i) != args {
            return Err(invariant());
        }
        self.value(i, ty)
    }
    pub fn branch(&mut self, condition: Value) -> Result<(Block, Block), Diagnostic> {
        let i = self.take()?;
        let data = &self.f.dfg.insts[i];
        let targets =
            data.branch_destination(&self.f.dfg.jump_tables, &self.f.dfg.exception_tables);
        if data.opcode() != O::Brif
            || self.f.dfg.inst_args(i).first() != Some(&condition)
            || targets.len() != 2
            || self.next.is_some()
            || targets.iter().any(|t| t.args(&self.f.dfg.value_lists).next().is_some())
        {
            return Err(invariant());
        }
        Ok((targets[0].block(&self.f.dfg.value_lists), targets[1].block(&self.f.dfg.value_lists)))
    }
    pub fn jump(&mut self, arguments: &[Value]) -> Result<Block, Diagnostic> {
        let i = self.take()?;
        let data = &self.f.dfg.insts[i];
        let targets =
            data.branch_destination(&self.f.dfg.jump_tables, &self.f.dfg.exception_tables);
        if data.opcode() != O::Jump
            || targets.len() != 1
            || self.next.is_some()
            || targets[0]
                .args(&self.f.dfg.value_lists)
                .map(|v| v.as_value())
                .ne(arguments.iter().copied().map(Some))
        {
            return Err(invariant());
        }
        Ok(targets[0].block(&self.f.dfg.value_lists))
    }
    pub fn unreachable(&mut self) -> Result<(), Diagnostic> {
        let i = self.take()?;
        if self.f.dfg.insts[i].opcode() != O::Trap
            || self.f.dfg.insts[i].trap_code() != Some(TrapCode::unwrap_user(7))
            || self.next.is_some()
        {
            return Err(invariant());
        }
        Ok(())
    }
}

impl Cursor<'_> {
    pub fn branch_args(
        &mut self,
        condition: Value,
        yes: &[Value],
        no: &[Value],
    ) -> Result<(Block, Block), Diagnostic> {
        let inst = self.take()?;
        let data = &self.f.dfg.insts[inst];
        let targets =
            data.branch_destination(&self.f.dfg.jump_tables, &self.f.dfg.exception_tables);
        if data.opcode() != O::Brif
            || self.f.dfg.inst_args(inst).first() != Some(&condition)
            || targets.len() != 2
            || self.next.is_some()
        {
            return Err(invariant());
        }
        for (target, args) in targets.iter().zip([yes, no]) {
            if target
                .args(&self.f.dfg.value_lists)
                .map(|value| value.as_value())
                .ne(args.iter().copied().map(Some))
            {
                return Err(invariant());
            }
        }
        Ok((targets[0].block(&self.f.dfg.value_lists), targets[1].block(&self.f.dfg.value_lists)))
    }
}
