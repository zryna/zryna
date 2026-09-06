use super::super::diagnostics::span;
use super::Ty;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;
use zryna_ir::data_ownership_v1::raw;

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn continued_index_step(
        &mut self,
        source: u32,
        index: u32,
        parent: Option<raw::BorrowId>,
        write: bool,
        integer: Ty,
    ) -> Option<raw::ValueId> {
        let at = span(
            self.decisions.input.sources(),
            self.decisions.function.body.expressions.get(index as usize)?.span,
        );
        let source = if parent.is_none() { Some(self.resolve(source)?) } else { None };
        if let Some(source) = source {
            self.available_vector(source, write, at)?;
        }
        let start = self.steps.len();
        self.push(
            Operation::IndexedEnter { end: usize::MAX, result: usize::MAX },
            integer,
            at,
            None,
        );
        let value = self.walk(index, integer)?;
        if let Some(parent) = parent {
            let borrow = raw::BorrowId(self.state.facts.next_borrow);
            let cleanup = self.reverse(integer, at)?;
            self.indexed_effect(
                raw::InstructionKind::ProjectIndexedBorrow {
                    parent,
                    borrow,
                    index: value,
                    cleanup,
                },
                integer,
                at,
            )?;
        } else {
            self.begin_access(source?, value, write, integer, at)?;
        }
        self.finish_continued_scope(start, value, integer, at)?;
        Some(value)
    }

    pub(super) fn continued_index_finish(
        &mut self,
        id: u32,
        ty: Ty,
        borrow: raw::BorrowId,
        replacement: bool,
    ) -> Option<raw::ValueId> {
        let at = span(
            self.decisions.input.sources(),
            self.decisions.function.body.expressions.get(id as usize)?.span,
        );
        if !replacement {
            self.visits = self.visits.checked_add(1)?;
        }
        let start = self.steps.len();
        self.push(Operation::IndexedEnter { end: usize::MAX, result: usize::MAX }, ty, at, None);
        let value = self.finish_chained_access(borrow, ty, at, replacement.then_some(id))?;
        self.finish_continued_scope(start, value, ty, at)?;
        Some(value)
    }

    fn finish_continued_scope(
        &mut self,
        start: usize,
        value: raw::ValueId,
        ty: Ty,
        at: zryna_source::Span,
    ) -> Option<()> {
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, ty, at, None);
        Some(())
    }
}
