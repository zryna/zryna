use super::{
    Body, Closed, Domain, Failure, State, Statement, UntrustedSpan, Value, charge, copy_key,
    exhausted, keys, originals, raw, reject, reserve,
};
mod matches;
mod upgrades;
impl<'s> Body<'_, 's> {
    pub(super) fn block_statements(&mut self, index: u32, depth: usize) -> Result<bool, Failure> {
        if depth
            > usize::try_from(zryna_syntax::v4::MAX_NESTING_DEPTH)
                .map_err(|_| Failure::InternalFailure)?
        {
            return Err(exhausted("source statement depth exceeds 128"));
        }
        let block = &self.original.body.blocks[index as usize];
        charge(self.spent, 1)?;
        self.regions.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.regions.push(block.span);
        charge(self.spent, block.statements.len())?;
        let mut terminal = false;
        for at in &block.statements {
            if terminal {
                return Err(reject("executable source contains a statement after terminal return"));
            }
            let statement = &self.original.body.statements[*at as usize];
            terminal = self.statement(statement, depth)?;
        }
        if !terminal && depth == 1 {
            if self.output.result_type != raw::ValueType::Unit {
                return Err(reject("source function lacks a complete return"));
            }
            self.end_created_loans(0, block.span)?;
            let cleanup = self.failure(block.span)?;
            self.output.blocks[self.block].terminator =
                raw::Terminator::Return { value: None, cleanup };
            terminal = true;
        }
        self.regions.pop().ok_or(Failure::InternalFailure)?;
        Ok(terminal)
    }
    fn statement(
        &mut self,
        statement: &'s zryna_syntax::v4::RawStatementSyntax,
        depth: usize,
    ) -> Result<bool, Failure> {
        let span = statement.span;
        match &statement.kind {
            Statement::LocalDeclaration { mutable, name, type_syntax, initializer, .. } => {
                let closed = originals::resolve(&self.resolver, *type_syntax, self.spent)?;
                if let Closed::Borrow(key, exclusive) = closed {
                    self.local_loan(
                        &name.text,
                        (&key, exclusive),
                        *mutable,
                        *initializer,
                        span,
                        depth,
                    )?;
                    return Ok(false);
                }
                let expected = self.closed_type(closed)?;
                let value = self.expression(*initializer, depth + 1)?;
                if value.ty != expected {
                    return Err(self.error(
                        span,
                        "ZRYNA-M7006",
                        "initializer differs from declared original type",
                    ));
                }
                let raw::ValueType::Stored(ty) = expected else {
                    return Err(reject("unit/loan local source replay remains open"));
                };
                // Stored local identity is function-wide; the live lexical stack
                // also contains parameters/loans and is restored at branch boundaries.
                let ordinal = self.next_local;
                let next_local = ordinal.checked_add(1).ok_or(Failure::InternalFailure)?;
                let p = self.place(ty, raw::PlaceKind::Local(ordinal), span)?;
                self.next_local = next_local;
                self.initialize_from(p, value, span)?;
                self.bind_mutable(&name.text, p, ty, *mutable)?;
            }
            Statement::Return { value, .. } => {
                let value = self.expression(*value, depth + 1)?;
                if value.ty != self.output.result_type {
                    return Err(reject("return differs from original result"));
                }
                self.consume(value, span)?;
                self.end_created_loans(0, span)?;
                let cleanup = self.failure(span)?;
                self.output.blocks[self.block].terminator = raw::Terminator::Return {
                    value: if value.ty == raw::ValueType::Unit { None } else { Some(value.id) },
                    cleanup,
                };
                return Ok(true);
            }
            Statement::ExpressionStatement { expression, .. } => {
                let value = self.expression(*expression, depth + 1)?;
                if let Some(p) = self.owners[value.id as usize] {
                    self.drop_place(p, span)?;
                }
            }
            Statement::Block { block } => {
                let locals = self.state.locals.len();
                let order = self.output.places.len();
                let terminal = self.block_statements(*block, depth + 1)?;
                if !terminal {
                    self.close_scope(locals, order, span)?;
                }
                return Ok(terminal);
            }
            Statement::If { condition, then_block, else_clause, .. } => {
                return self.branch(
                    *condition,
                    *then_block,
                    else_clause.as_ref().map(|c| c.block),
                    span,
                    depth,
                );
            }
            Statement::WeakUpgrade { .. } => return self.upgrade_statement(statement, depth),
            Statement::Assignment { target, value, .. } => {
                self.assign_source(*target, *value, span, depth)?;
            }
            Statement::While { condition, body_block, .. } => {
                self.source_loop(*condition, *body_block, span, depth)?;
            }
        }
        Ok(false)
    }
    pub(super) fn initialize(&mut self, p: u32) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        self.tree_whole(p)?;
        if !self.copy(self.output.places[p as usize].ty)? {
            charge(self.spent, 1)?;
            self.state.order.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            self.state.order.push(p);
        }
        Ok(())
    }
    pub(super) fn drop_place(&mut self, p: u32, span: UntrustedSpan) -> Result<(), Failure> {
        let kind = self.tree_status(p)?.0;
        if kind != super::tree::Init::Empty {
            self.drop_source_tree(p, kind, span)?;
        }
        Ok(())
    }
    pub(super) fn close_scope(
        &mut self,
        locals: usize,
        order: usize,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        self.end_scope_loans(locals)?;
        self.close_scope_storage(locals, order, span)
    }
    pub(super) fn close_scope_storage(
        &mut self,
        locals: usize,
        order: usize,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        // The new roots are released in reverse initialization order. Moving an
        // outer root never creates a second root under its old lexical name.
        charge(self.spent, self.state.order.len())?;
        let mut at = self.state.order.len();
        while at > 0 {
            at -= 1;
            let p = self.state.order[at];
            if p as usize >= order {
                self.drop_place(p, span)?;
            }
        }
        // Copy roots have no drop-plan entry, but their storage lifetime still ends.
        // Otherwise branch-only Copy locals would remain initialized at a shared join.
        charge(self.spent, self.output.places.len() - order)?;
        for at in (order..self.output.places.len()).rev() {
            if self.tree_parent(u32::try_from(at).map_err(|_| Failure::InternalFailure)?).is_none()
                && (self.state.live[at] || self.state.partial[at])
            {
                self.drop_place(u32::try_from(at).map_err(|_| Failure::InternalFailure)?, span)?;
            }
        }
        self.state.locals.truncate(locals);
        Ok(())
    }
    fn branch(
        &mut self,
        condition: u32,
        yes: u32,
        no: Option<u32>,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<bool, Failure> {
        let cond = self.expression(condition, depth + 1)?;
        if cond.ty != self.primitive(0)? {
            return Err(reject("if requires bool"));
        }
        let entry = self.block;
        let saved = self.snapshot()?;
        let yes_block = self.new_block()?;
        let no_block = self.new_block()?;
        let locals = saved.locals.len();
        let order = saved.live.len();
        self.block = yes_block;
        let yes_terminal = self.block_statements(yes, depth + 1)?;
        if !yes_terminal {
            self.close_scope(locals, order, span)?;
        }
        let yes_end = self.block;
        let yes_state = self.snapshot()?;
        self.restore(saved)?;
        self.block = no_block;
        let no_terminal =
            if let Some(at) = no { self.block_statements(at, depth + 1)? } else { false };
        if !no_terminal {
            self.close_scope(locals, order, span)?;
        }
        let no_end = self.block;
        self.output.blocks[entry].terminator = raw::Terminator::Branch {
            condition: cond.id,
            yes: raw::Edge {
                target: u32::try_from(yes_block).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
            no: raw::Edge {
                target: u32::try_from(no_block).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
        };
        self.finish_alternatives(yes_terminal, no_terminal, yes_end, no_end, yes_state)
    }
    pub(super) fn finish_alternatives(
        &mut self,
        yes_terminal: bool,
        no_terminal: bool,
        yes_end: usize,
        no_end: usize,
        yes_state: State<'s>,
    ) -> Result<bool, Failure> {
        match (yes_terminal, no_terminal) {
            (true, true) => Ok(true),
            (false, true) => {
                self.restore(yes_state)?;
                self.block = yes_end;
                Ok(false)
            }
            (true, false) => Ok(false),
            (false, false) => {
                self.same_state(&yes_state)?;
                let join = self.new_block()?;
                let target = u32::try_from(join).map_err(|_| Failure::InternalFailure)?;
                self.output.blocks[yes_end].terminator =
                    raw::Terminator::Jump(raw::Edge { target, arguments: Vec::new() });
                self.output.blocks[no_end].terminator =
                    raw::Terminator::Jump(raw::Edge { target, arguments: Vec::new() });
                self.block = join;
                Ok(false)
            }
        }
    }
    pub(super) fn same_state(&mut self, other: &State<'_>) -> Result<(), Failure> {
        self.compare_source_state(other)
    }
}
