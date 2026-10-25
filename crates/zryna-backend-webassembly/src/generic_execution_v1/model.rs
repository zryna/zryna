//! Retained new-domain views and fixed helper indices; no old IDs are converted.
use super::{Diagnostic, budget, invariant, memory, work};
mod accounting;
pub(super) mod containers;
mod continuation;
mod discovery;
mod loans;
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCloneNode, VerifiedExecution, VerifiedFunction, raw,
};
use zryna_layout::{StorageTarget, TypeCategory, generic_v1::TypeView};
pub(super) const COPY: u32 = 0;
pub(super) const STRING_FROM: u32 = 1;
pub(super) const STRING_CLONE: u32 = 2;
pub(super) const STRING_RELEASE: u32 = 3;
pub(super) const ALLOCATE: u32 = 4;
pub(super) const HELPER_BASE: u32 = 5;
pub(super) struct Model<'p, 'a, 'm> {
    accounting: Option<accounting::Plan<'p, 'a>>,
    pub(super) work: std::cell::Cell<usize>,
    pub(super) writer: std::cell::Cell<usize>,
    pub(super) program: &'p VerifiedExecution<'a>,
    pub(super) memory: &'m memory::VerifiedTargetMemoryPlan<'p, 'a>,
    pub(super) functions: Vec<VerifiedFunction<'p, 'a>>,
    pub(super) types: Vec<TypeView<'a>>,
    pub(super) clones: Vec<Option<VerifiedCloneNode<'p, 'a>>>,
    pub(super) source_base: u32,
    pub(super) wrapper_base: u32,
    pub(super) observer: u32,
    pub(super) recorder: u32,
    pub(super) container_base: u32,
    pub(super) container_count: u32,
    pub(super) continuation: bool,
    pub(super) concat: bool,
}
impl<'p, 'a, 'm> Model<'p, 'a, 'm> {
    pub(super) fn new(
        program: &'p VerifiedExecution<'a>,
        memory: &'m memory::VerifiedTargetMemoryPlan<'p, 'a>,
    ) -> Result<Self, Diagnostic> {
        if !memory.is_bound_to(program) {
            return Err(invariant());
        }
        let mut work = 0;
        let (types, functions, clones) = discovery::collect(program, &mut work)?;
        let source_base = HELPER_BASE
            .checked_add(
                u32::try_from(types.len())
                    .map_err(|_| budget())?
                    .checked_mul(2)
                    .ok_or_else(budget)?,
            )
            .ok_or_else(budget)?;
        let wrapper_base = source_base
            .checked_add(u32::try_from(functions.len()).map_err(|_| budget())?)
            .ok_or_else(budget)?;
        let observer = wrapper_base
            .checked_add(u32::try_from(program.export_functions().len()).map_err(|_| budget())?)
            .ok_or_else(budget)?;
        let recorder = observer.checked_add(1).ok_or_else(budget)?;
        let container_base = observer
            .checked_add(if memory.claims().observed { 2 } else { 0 })
            .ok_or_else(budget)?;
        work::take(&mut work, types.len())?;
        let (continuation, concat) = continuations(&functions, &mut work)?;
        let container_count = if types.iter().any(|t| {
            matches!(t.category(), TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak)
        }) {
            if continuation { 12 } else { 10 }
        } else {
            0
        };
        container_base
            .checked_add(container_count)
            .and_then(|n| n.checked_add(if concat { 2 } else { 0 }))
            .ok_or_else(budget)?;
        let accounting = accounting::Plan::derive(
            program,
            &types,
            &functions,
            memory.claims().observed,
            &mut work,
        )?;
        container_base
            .checked_add(container_count)
            .and_then(|n| n.checked_add(if concat { 2 } else { 0 }))
            .and_then(|n| {
                n.checked_add(
                    accounting.as_ref().map_or(0, |plan| plan.count(concat, continuation)),
                )
            })
            .ok_or_else(budget)?;
        let model = Self {
            accounting,
            work: std::cell::Cell::new(work),
            writer: std::cell::Cell::new(memory.static_data_bytes() as usize),
            program,
            memory,
            functions,
            types,
            clones,
            source_base,
            wrapper_base,
            observer,
            recorder,
            container_base,
            container_count,
            continuation,
            concat,
        };
        model.runtime()?;
        Ok(model)
    }
    fn runtime(&self) -> Result<(), Diagnostic> {
        self.check_runtime_declarations()
    }
    pub(super) fn clone_index(&self, ty: u32) -> u32 {
        HELPER_BASE + self.types[ty as usize].id().index()
    }
    pub(super) fn drop_index(&self, ty: u32) -> u32 {
        HELPER_BASE + (self.source_base - HELPER_BASE) / 2 + ty
    }
    pub(super) fn take(&self, units: usize) -> Result<(), Diagnostic> {
        let mut work = self.work.get();
        work::take(&mut work, units)?;
        self.work.set(work);
        Ok(())
    }
    pub(super) fn observed(&self) -> bool {
        self.memory.claims().observed
    }
}

// Shared production scan, extracted unchanged so its actual header debit is privately testable.
fn continuations(
    functions: &[VerifiedFunction<'_, '_>],
    work: &mut usize,
) -> Result<(bool, bool), Diagnostic> {
    work::take(work, functions.len())?;
    let mut continuation = false;
    let mut concat = false;
    for function in functions {
        #[cfg(test)]
        header_tests::visit();
        work::take(work, function.blocks().len())?;
        for block in function.blocks() {
            work::take(work, block.operations().len())?;
            concat |= block
                .operations()
                .any(|op| matches!(op.kind(), raw::Operation::StringConcat { .. }));
            work::take(work, block.operations().len())?;
            continuation |= block.weak_upgrade().is_some()
                || block.operations().any(|op| op.vec_push().is_some());
        }
    }
    Ok((continuation, concat))
}

#[cfg(test)]
mod header_tests;
