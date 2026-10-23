//! Exact success-only Shared owner, no synthetic Option or unselected branch state.
use super::{Body, Failure, Statement, charge, raw, reject};
use zryna_syntax::v4::{RawIdentifierSyntax, RawStatementSyntax};
impl<'s> Body<'_, 's> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn upgrade_statement(
        &mut self,
        statement: &'s RawStatementSyntax,
        depth: usize,
    ) -> Result<bool, Failure> {
        let Statement::WeakUpgrade { weak, binding, success_block, failure_block, .. } =
            &statement.kind
        else {
            return Err(Failure::InternalFailure);
        };
        let weak_span = self.original.body.expressions[*weak as usize].span;
        let mark = self.output.loans.len();
        let (source, temporary) = self.observed(*weak, weak_span, depth)?;
        charge(self.spent, 2)?;
        if self.world.kind(source.ty) != Some(super::super::Kind::Weak) {
            return Err(reject("upgrade requires exact source Weak"));
        }
        let payload = self
            .world
            .arguments(source.ty)
            .and_then(|a| a.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        let shared = self.container_type(0x22, payload)?;
        let failure = self.failure(weak_span)?;
        let entry = self.block;
        let saved = self.snapshot()?;
        let locals = saved.locals.len();
        let start = saved.live.len();
        let yes_block = self.new_block()?;
        let no_block = self.new_block()?;
        self.block = yes_block;
        let generated = self.upgrade_binding(binding, shared)?;
        if temporary {
            self.drop_observed(&source, weak_span)?;
        }
        self.end_created_loans(mark, weak_span)?;
        let yes_terminal = self.block_statements(*success_block, depth + 1)?;
        if !yes_terminal {
            self.close_scope(locals, start, statement.span)?;
        }
        let yes_end = self.block;
        let yes_state = self.snapshot()?;
        self.restore(saved)?;
        self.block = no_block;
        if temporary {
            self.drop_observed(&source, weak_span)?;
        }
        self.end_created_loans(mark, weak_span)?;
        let no_terminal = self.block_statements(*failure_block, depth + 1)?;
        if !no_terminal {
            self.close_scope(locals, start, statement.span)?;
        }
        let no_end = self.block;
        self.output.blocks[entry].terminator = raw::Terminator::WeakUpgradeBranch {
            weak: source.source,
            success_binding: generated,
            success: raw::Edge {
                target: u32::try_from(yes_block).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
            expired: raw::Edge {
                target: u32::try_from(no_block).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
            failure,
        };
        self.finish_alternatives(yes_terminal, no_terminal, yes_end, no_end, yes_state)
    }
    fn upgrade_binding(
        &mut self,
        name: &'s RawIdentifierSyntax,
        shared: u32,
    ) -> Result<raw::ValueDefinition, Failure> {
        let d = self.definition(raw::ValueType::Stored(shared), true, name.span)?;
        charge(self.spent, 1)?;
        self.output.blocks[self.block]
            .value_parameters
            .try_reserve(1)
            .map_err(|_| Failure::AllocationFailure)?;
        self.output.blocks[self.block].value_parameters.push(d.clone());
        self.activate(d.id)?;
        let place = self.owners[d.id as usize].ok_or(Failure::InternalFailure)?;
        self.bind(&name.text, place, shared)?;
        Ok(d)
    }
}
