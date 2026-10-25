//! Canonical source-to-place replay for the first full-profile vertical; no old execution seal.
use super::{
    Closed, Expr, Failure, Kind, Originals, Resolver, SourceMap, Target, TypeWorld, UntrustedSpan,
    charge, copy_key, diagnostic, exhausted, lookup, originals, raw, reject, reserve, v5,
};
use crate::generic_v1::keys::{self, Domain};
use zryna_syntax::v4::RawStatementKind as Statement;
mod anchors;
mod assignment;
mod calls;
mod constructors;
mod containers;
mod context;
mod expressions;
mod finalize;
mod flow;
mod indexed;
mod lifecycle;
mod loans;
mod loops;
mod places;
mod push;
mod state_compare;
mod static_index;
mod strings;
mod tree;
#[derive(Clone, Copy)]
struct Value {
    id: u32,
    ty: raw::ValueType,
}
#[derive(Clone)]
struct Binding<'s> {
    name: &'s str,
    source: raw::PlaceOrLoan,
    ty: u32,
    mutable: bool,
}
#[derive(Clone)]
struct State<'s> {
    locals: Vec<Binding<'s>>,
    live: Vec<bool>,
    partial: Vec<bool>,
    order: Vec<u32>,
    loans: Vec<bool>,
    loan_order: Vec<u32>,
}
struct DeferredCall {
    block: usize,
    position: usize,
    span: UntrustedSpan,
    actions: Vec<raw::CleanupAction>,
    target: usize,
}
struct Body<'p, 's> {
    program: &'p raw::Program,
    originals: &'p Originals<'s>,
    sources: &'p SourceMap,
    world: &'p TypeWorld,
    resolver: Resolver<'p, 's>,
    original: &'s v5::RawFunctionSyntax,
    function: usize,
    output: raw::Function,
    block: usize,
    next: u32,
    next_local: u32,
    owners: Vec<Option<u32>>,
    children: Vec<Vec<u32>>,
    expanded: Vec<bool>,
    loan_scopes: Vec<usize>,
    loan_places: Vec<Option<u32>>,
    regions: Vec<UntrustedSpan>,
    state: State<'s>,
    used: &'p mut [bool],
    calls: Vec<DeferredCall>,
    match_uses: Vec<super::proof::matches::Use>,
    spent: &'p mut usize,
}
pub(super) fn produce(
    program: &mut raw::Program,
    originals: &Originals<'_>,
    sources: &SourceMap,
    world: &TypeWorld,
    spent: &mut usize,
) -> Result<super::proof::matches::Matches, Failure> {
    let mut match_sites = super::proof::matches::Matches::new(program.functions.len(), spent)?;
    charge(spent, program.type_keys.len())?;
    let mut used = reserve(program.type_keys.len())?;
    used.resize(program.type_keys.len(), false);
    for key in originals::stored_roots(originals, spent)? {
        let id = lookup(&program.type_keys, &key, spent)?;
        used[id as usize] = true;
    }
    charge(spent, program.functions.len())?;
    let mut calls = reserve(program.functions.len())?;
    for fi in 0..program.functions.len() {
        charge(spent, 1)?;
        let (function, deferred, matches) = {
            let mut body = Body::new(program, originals, sources, world, fi, &mut used, spent)?;
            body.block_statements(body.original.body.root_block, 1)?;
            body.finish()?
        };
        match_sites.function(&function, matches, spent)?;
        program.functions[fi] = function;
        calls.push(deferred);
    }
    finalize::calls(program, &calls, spent)?;
    finalize::demand(program, world, &mut used, spent)?;
    Ok(match_sites)
}
impl Body<'_, '_> {
    fn place(
        &mut self,
        ty: u32,
        kind: raw::PlaceKind,
        span: UntrustedSpan,
    ) -> Result<u32, Failure> {
        charge(self.spent, size_of::<Vec<u32>>() + 4)?;
        if self.output.places.len() >= crate::data_ownership_v1::MAX_PLACES_PER_FUNCTION {
            return Err(exhausted("source static place arena exceeds inherited limit"));
        }
        let id = u32::try_from(self.output.places.len()).map_err(|_| Failure::InternalFailure)?;
        self.output.places.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.live.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.partial.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.children.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.expanded.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.output.places.push(raw::Place { id, ty, span, kind });
        self.state.live.push(false);
        self.state.partial.push(false);
        self.children.push(Vec::new());
        self.expanded.push(false);
        Ok(id)
    }
    fn definition(
        &mut self,
        ty: raw::ValueType,
        temporary: bool,
        span: UntrustedSpan,
    ) -> Result<raw::ValueDefinition, Failure> {
        charge(self.spent, 1)?;
        self.owners.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or(Failure::InternalFailure)?;
        self.owners.push(None);
        if temporary
            && let raw::ValueType::Stored(t) = ty
            && !self.copy(t)?
        {
            let p = self.place(t, raw::PlaceKind::Temporary(id), span)?;
            self.owners[id as usize] = Some(p);
        }
        Ok(raw::ValueDefinition { id, ty })
    }
    fn activate(&mut self, id: u32) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        if let Some(p) = self.owners[id as usize] {
            charge(self.spent, 1)?;
            self.state.order.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            self.state.live[p as usize] = true;
            self.state.order.push(p);
        }
        Ok(())
    }
    fn instruction(
        &mut self,
        result: Option<raw::ValueDefinition>,
        operation: raw::Operation,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        let list = &mut self.output.blocks[self.block].instructions;
        list.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        list.push(raw::Instruction { result, operation, site_span: span });
        Ok(())
    }
    fn emit(
        &mut self,
        ty: raw::ValueType,
        operation: raw::Operation,
        span: UntrustedSpan,
    ) -> Result<Value, Failure> {
        let d = self.definition(ty, true, span)?;
        let value = Value { id: d.id, ty };
        self.instruction(Some(d), operation, span)?;
        self.activate(value.id)?;
        Ok(value)
    }
    fn consume_place(&mut self, place: u32, span: UntrustedSpan) -> Result<(), Failure> {
        self.complete_place(place, span)?;
        if !self.copy(self.output.places[place as usize].ty)? {
            self.tree_take(place, span)?;
        }
        Ok(())
    }
    fn consume(&mut self, v: Value, span: UntrustedSpan) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        if let Some(p) = self.owners[v.id as usize] {
            self.consume_place(p, span)?;
        }
        Ok(())
    }
    fn plan(
        &mut self,
        actions: Vec<raw::CleanupAction>,
        span: UntrustedSpan,
    ) -> Result<u32, Failure> {
        charge(self.spent, 1)?;
        let id =
            u32::try_from(self.output.cleanup_plans.len()).map_err(|_| Failure::InternalFailure)?;
        self.output.cleanup_plans.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.output.cleanup_plans.push(raw::CleanupPlan { id, span, actions });
        Ok(id)
    }
    fn failure(&mut self, span: UntrustedSpan) -> Result<u32, Failure> {
        let a = self.actions()?;
        self.plan(a, span)
    }
    fn new_block(&mut self) -> Result<usize, Failure> {
        charge(self.spent, 1)?;
        let id = self.output.blocks.len();
        self.output.blocks.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.output.blocks.push(raw::Block {
            id: u32::try_from(id).map_err(|_| Failure::InternalFailure)?,
            value_parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: raw::Terminator::Trap {
                identity: raw::TrapIdentity::Allocation,
                cleanup: u32::MAX,
            },
        });
        Ok(id)
    }
    fn finish(
        self,
    ) -> Result<(raw::Function, Vec<DeferredCall>, Vec<super::proof::matches::Use>), Failure> {
        charge(self.spent, self.output.blocks.len())?;
        if self
            .output
            .blocks
            .iter()
            .any(|b| matches!(b.terminator, raw::Terminator::Trap { cleanup: u32::MAX, .. }))
        {
            return Err(reject("closed source function lacks a complete return path"));
        }
        Ok((self.output, self.calls, self.match_uses))
    }
}
