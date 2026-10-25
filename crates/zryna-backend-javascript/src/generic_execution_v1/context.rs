//! Retained same-seal indices; every allocation and repeated lookup is charged.
use super::{emission_budget, internal};
mod containers;
use containers::Containers;
use std::{cell::Cell, mem::size_of};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    CloneNodeKind, VerifiedBlock, VerifiedCloneNode, VerifiedExecution, VerifiedFunction,
    VerifiedLoan, VerifiedOperation, VerifiedParameter, VerifiedPlace, raw,
};
use zryna_layout::{StorageTarget, generic_v1::TypeView};
pub(super) struct Context<'p, 'a> {
    pub(super) accounting: Option<super::runtime::accounting::Plan<'p, 'a>>,
    pub(super) program: &'p VerifiedExecution<'a>,
    pub(super) functions: Vec<Function<'p, 'a>>,
    pub(super) types: Vec<TypeView<'a>>,
    pub(super) clones: Vec<Option<VerifiedCloneNode<'p, 'a>>>,
    pub(super) copies: Vec<bool>,
    pub(super) work: Cell<usize>,
    pub(super) containers: Containers,
}
pub(super) struct Function<'p, 'a> {
    pub(super) view: VerifiedFunction<'p, 'a>,
    pub(super) parameters: Vec<VerifiedParameter<'p, 'a>>,
    pub(super) blocks: Vec<VerifiedBlock<'p, 'a>>,
    pub(super) places: Vec<VerifiedPlace<'p, 'a>>,
    pub(super) loans: Vec<VerifiedLoan<'p, 'a>>,
    pub(super) values: Vec<raw::ValueType>,
}
pub(super) fn charge(work: &Cell<usize>, count: usize) -> Result<(), Diagnostic> {
    work.set(
        work.get()
            .checked_add(count)
            .filter(|n| *n <= crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
            .ok_or_else(emission_budget)?,
    );
    Ok(())
}
fn collect<T>(
    values: impl ExactSizeIterator<Item = T>,
    work: &Cell<usize>,
) -> Result<Vec<T>, Diagnostic> {
    charge(work, values.len().checked_mul(size_of::<T>().max(1)).ok_or_else(emission_budget)?)?;
    let mut result = Vec::new();
    result.try_reserve_exact(values.len()).map_err(|_| emission_budget())?;
    result.extend(values);
    Ok(result)
}
impl<'p, 'a> Context<'p, 'a> {
    pub(super) fn new(program: &'p VerifiedExecution<'a>) -> Result<Self, Diagnostic> {
        let work = Cell::new(0);
        let types = collect(program.layouts(StorageTarget::Linear32V1).types(), &work)?;
        let containers = Containers::new(program, &types, &work)?;
        let copies = collect((0..types.len()).map(|_| false), &work)?;
        let clones = collect((0..types.len()).map(|_| None), &work)?;
        charge(
            &work,
            program
                .functions()
                .len()
                .checked_mul(size_of::<Function<'p, 'a>>())
                .ok_or_else(emission_budget)?,
        )?;
        let mut functions = Vec::new();
        functions.try_reserve_exact(program.functions().len()).map_err(|_| emission_budget())?;
        for function in program.functions() {
            functions.push(Function::new(function, &work)?);
        }
        // Function views are already sealed; no raw model is reconstructed for emission.
        let mut context =
            Self { program, functions, types, clones, copies, work, containers, accounting: None };
        context.mark_operations()?;
        Ok(context)
    }
    pub(super) fn ty(&self, id: u32) -> Result<TypeView<'a>, Diagnostic> {
        charge(&self.work, 1)?;
        self.types.get(id as usize).copied().ok_or_else(internal)
    }
    fn mark_operations(&mut self) -> Result<(), Diagnostic> {
        for operation in self.program.operations() {
            charge(&self.work, 1)?;
            if let Some(clone) = operation.clone_frontier() {
                self.mark_clone(clone.frontier(), 0)?;
            }
            let function = self.functions.get(operation.site().function).ok_or_else(internal)?;
            if let Some(proof) = operation.drop_targets()
                && !proof.is_whole()
            {
                charge(
                    &self.work,
                    proof
                        .targets()
                        .len()
                        .checked_add(1)
                        .and_then(|n| n.checked_mul(function.places.len().checked_add(8)?))
                        .and_then(|n| n.checked_mul(32))
                        .ok_or_else(emission_budget)?,
                )?;
            }
            let copy = match operation.kind() {
                raw::Operation::ReadPlace { place, mode: raw::ReadMode::Copy } => {
                    Some(function.place(*place)?.ty())
                }
                raw::Operation::ReadLoan { loan } => Some(function.loan(*loan)?.referent()),
                raw::Operation::IndexCopy { .. } => operation.result().and_then(|v| match v.ty() {
                    raw::ValueType::Stored(ty) => Some(ty),
                    raw::ValueType::Unit => None,
                }),
                _ => None,
            };
            if let Some(ty) = copy {
                self.mark_copy(ty, 0)?;
            }
        }
        Ok(())
    }
    fn mark_clone(
        &mut self,
        node: VerifiedCloneNode<'p, 'a>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        charge(&self.work, 1)?;
        if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
            return Err(emission_budget());
        }
        let slot = self.clones.get_mut(node.ty() as usize).ok_or_else(internal)?;
        if slot.is_some() {
            return Ok(());
        }
        *slot = Some(node);
        match node.kind() {
            CloneNodeKind::Copy => self.mark_copy(node.ty(), depth + 1)?,
            CloneNodeKind::String | CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {}
            CloneNodeKind::Struct => {
                let fields = node.fields();
                charge(&self.work, fields.len())?;
                for (_, child) in fields {
                    self.mark_clone(child, depth + 1)?;
                }
            }
            CloneNodeKind::Enum => {
                let variants = node.variants();
                charge(&self.work, variants.len())?;
                for (_, child) in variants {
                    if let Some(child) = child {
                        self.mark_clone(child, depth + 1)?;
                    }
                }
            }
            CloneNodeKind::FixedArray { .. } | CloneNodeKind::VecEach => {
                self.mark_clone(node.element().ok_or_else(internal)?, depth + 1)?;
            }
        }
        Ok(())
    }
    fn mark_copy(&mut self, id: u32, depth: usize) -> Result<(), Diagnostic> {
        use zryna_layout::TypeCategory as Category;
        charge(&self.work, 1)?;
        if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
            return Err(emission_budget());
        }
        let slot = self.copies.get_mut(id as usize).ok_or_else(internal)?;
        if *slot {
            return Ok(());
        }
        *slot = true;
        let ty = self.ty(id)?;
        match ty.category() {
            Category::Bool | Category::I32 => {}
            Category::Struct => {
                let fields = ty.fields();
                charge(&self.work, fields.len())?;
                for (_, child, _) in fields {
                    self.mark_copy(child.index(), depth + 1)?;
                }
            }
            Category::Enum => {
                let variants = ty.variants();
                charge(&self.work, variants.len())?;
                for (_, child) in variants {
                    if let Some(child) = child {
                        self.mark_copy(child.index(), depth + 1)?;
                    }
                }
            }
            Category::FixedArray => {
                self.mark_copy(ty.referenced_type().ok_or_else(internal)?.index(), depth + 1)?;
            }
            _ => return Err(internal()),
        }
        Ok(())
    }
}
impl<'p, 'a> Function<'p, 'a> {
    fn new(view: VerifiedFunction<'p, 'a>, work: &Cell<usize>) -> Result<Self, Diagnostic> {
        let parameters = collect(view.parameters(), work)?;
        let blocks = collect(view.blocks(), work)?;
        let places = collect(view.places(), work)?;
        let loans = collect(view.loans(), work)?;
        let mut count =
            parameters.iter().filter(|p| matches!(p, VerifiedParameter::Value(_))).count();
        for block in &blocks {
            charge(
                work,
                block
                    .operations()
                    .len()
                    .checked_add(block.value_parameters().len())
                    .ok_or_else(emission_budget)?,
            )?;
            count = count
                .checked_add(block.value_parameters().len())
                .and_then(|n| {
                    n.checked_add(block.operations().filter(|o| o.result().is_some()).count())
                })
                .ok_or_else(emission_budget)?;
        }
        let mut values = collect((0..count).map(|_| raw::ValueType::Unit), work)?;
        for parameter in &parameters {
            if let VerifiedParameter::Value(value) = parameter {
                *values.get_mut(value.id() as usize).ok_or_else(internal)? = value.ty();
            }
        }
        for block in &blocks {
            for value in block
                .value_parameters()
                .chain(block.operations().filter_map(VerifiedOperation::result))
            {
                *values.get_mut(value.id() as usize).ok_or_else(internal)? = value.ty();
            }
        }
        Ok(Self { view, parameters, blocks, places, loans, values })
    }
    pub(super) fn place(&self, id: u32) -> Result<VerifiedPlace<'p, 'a>, Diagnostic> {
        self.places.get(id as usize).copied().ok_or_else(internal)
    }
    pub(super) fn loan(&self, id: u32) -> Result<VerifiedLoan<'p, 'a>, Diagnostic> {
        self.loans.get(id as usize).copied().ok_or_else(internal)
    }
}

#[cfg(test)]
mod tests;
