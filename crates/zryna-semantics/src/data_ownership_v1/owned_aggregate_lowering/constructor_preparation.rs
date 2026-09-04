use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::Ty;
use super::super::owned_constructor_plan::ConstructorKind;
use super::PrivateOwnedAggregateLowerer;
use super::constructor_resources::ConstructorCommitReservation;
use super::expression_decisions::{
    ArrayDecision, ExpressionDecisions, ExpressionKind, StructDecision,
};
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, Operation, PreparationPlan};
use super::preparation_state::PreparationState;

#[path = "preparation_execution.rs"]
mod execution;

#[cfg(test)]
#[path = "../tests/constructor_preparation_consumption.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/constructor_preparation_controls.rs"]
mod controls;

enum Children<'f> {
    Struct(StructDecision),
    Array(ArrayDecision<'f>),
    Enum(Option<(u32, Ty)>),
}

struct ConstructorFrame<'f> {
    children: Children<'f>,
    ty: Ty,
    at: Span,
    kind: ConstructorKind,
    next: usize,
    values: Vec<raw::ValueId>,
    reservation: ConstructorCommitReservation,
    start: usize,
    waiting: bool,
}

enum Frame<'f> {
    Visit(u32, Ty),
    Constructor(ConstructorFrame<'f>),
}

impl<'f> PreparationContext<'_, 'f, '_, '_> {
    fn enter(
        &mut self,
        children: Children<'f>,
        ty: Ty,
        at: Span,
        kind: ConstructorKind,
    ) -> Option<Frame<'f>> {
        let arity = match &children {
            Children::Struct(decision) => decision.children.len(),
            Children::Array(decision) => decision.elements.len(),
            Children::Enum(payload) => usize::from(payload.is_some()),
        };
        if !self.state.usage().constructor(ty, arity, at, self.decisions.errors) {
            return None;
        }
        let reservation =
            self.state.ledger().acquire_constructor(arity, usize::from(!ty.is_copy()))?;
        let start = self.steps.len();
        self.push(Operation::Enter { arity, kind, end: usize::MAX }, ty, at, None);
        Some(Frame::Constructor(ConstructorFrame {
            children,
            ty,
            at,
            kind,
            next: 0,
            values: Vec::with_capacity(arity),
            reservation,
            start,
            waiting: false,
        }))
    }

    fn walk(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let mut frames = vec![Frame::Visit(id, expected)];
        let mut result = None;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Visit(id, ty) => {
                    self.visits = self.visits.checked_add(1)?;
                    let decision = self.decisions.classify(id, ty)?;
                    let at = decision.at;
                    result = match decision.kind {
                        ExpressionKind::Bool(value) => self.emit_leaf(Leaf::Bool(value), ty, at),
                        ExpressionKind::I32(value) => self.emit_leaf(Leaf::I32(value), ty, at),
                        ExpressionKind::String(bytes) => {
                            let cleanup = self.reverse(ty, at)?;
                            self.emit_leaf(Leaf::String { bytes, cleanup }, ty, at)
                        }
                        ExpressionKind::Reference(name) => self.reference(name, ty, at),
                        ExpressionKind::Projection(id) => self.projection(id, ty, at),
                        ExpressionKind::StringClone(id) => self.string_clone(id, ty, at),
                        ExpressionKind::AggregateClone(id) => self.aggregate_clone(id, ty, at),
                        ExpressionKind::Struct(decision) => {
                            frames.push(self.enter(
                                Children::Struct(decision),
                                ty,
                                at,
                                ConstructorKind::Struct,
                            )?);
                            continue;
                        }
                        ExpressionKind::Array(decision) => {
                            frames.push(self.enter(
                                Children::Array(decision),
                                ty,
                                at,
                                ConstructorKind::FixedArray,
                            )?);
                            continue;
                        }
                        ExpressionKind::Enum(decision) => {
                            let kind = ConstructorKind::Enum {
                                variant: u32::try_from(decision.ordinal).ok()?,
                            };
                            frames.push(self.enter(
                                Children::Enum(decision.payload_input),
                                ty,
                                at,
                                kind,
                            )?);
                            continue;
                        }
                    };
                    result?;
                }
                Frame::Constructor(mut frame) => {
                    if frame.waiting {
                        frame.values.push(result.take()?);
                    }
                    let next = match &frame.children {
                        Children::Struct(decision) => match decision.children.get(frame.next) {
                            Some(&(syntax, expression)) => {
                                Some((expression, self.decisions.child_type(syntax)?))
                            }
                            None => None,
                        },
                        Children::Array(decision) => {
                            decision.elements.get(frame.next).map(|&id| (id, decision.element))
                        }
                        Children::Enum(payload) => {
                            if frame.next == 0 {
                                *payload
                            } else {
                                None
                            }
                        }
                    };
                    if let Some((id, ty)) = next {
                        frame.next = frame.next.checked_add(1)?;
                        frame.waiting = true;
                        frames.push(Frame::Constructor(frame));
                        frames.push(Frame::Visit(id, ty));
                    } else {
                        self.state.ledger().release_constructor(frame.reservation);
                        self.push(Operation::Release, frame.ty, frame.at, None);
                        result = Some(self.commit(frame.ty, frame.at, frame.kind, frame.values)?);
                        let end = self.steps.len();
                        let Operation::Enter { end: slot, .. } =
                            &mut self.steps[frame.start].operation
                        else {
                            unreachable!("constructor frame retains its own entry");
                        };
                        *slot = end;
                    }
                }
            }
        }
        result
    }
}

// The exclusive borrow binds preparation and consumption to one real lowerer state.
// Rejection drops scratch metadata only; no rollback of real arenas or cache is needed.
pub(super) struct PreparedValue<'l, 'a, 'f, 'e> {
    lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
    plan: PreparationPlan<'f>,
}

impl<'l, 'a, 'f, 'e> PreparedValue<'l, 'a, 'f, 'e> {
    pub(super) fn prepare(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
    ) -> Option<Self> {
        let start = lowerer.preparation_checkpoint();
        let storage = lowerer.preparation_storage();
        let mut context = PreparationContext {
            decisions: ExpressionDecisions {
                input: lowerer.input,
                file: lowerer.file,
                function: lowerer.function,
                module: lowerer.module,
                declarations: lowerer.declarations,
                graph: lowerer.graph,
                node_types: lowerer.node_types,
                layouts: lowerer.layouts,
                errors: lowerer.errors,
            },
            bindings: &lowerer.bindings,
            state: PreparationState {
                original_places: &lowerer.places,
                places: Vec::new(),
                projections: lowerer.projections.clone(),
                moved: lowerer.moved_projections.clone(),
                partial: lowerer.partial_roots.clone(),
                owners: lowerer.owners.clone(),
                counts: start.counts,
                storage,
                transitions: lowerer.reserved_transitions,
                types: lowerer.constructor_types.observed_snapshot(&lowerer.instructions),
                cache: lowerer.constructor_types.checkpoint(),
            },
            aggregate_subobject_moves: lowerer.aggregate_subobject_moves,
            steps: Vec::new(),
            visits: 0,
        };
        let result = context.walk(id, expected)?;
        let plan = PreparationPlan {
            start,
            steps: context.steps,
            result,
            result_type: expected,
            owners: context.state.owners,
            projections: context.state.projections,
            moved: context.state.moved,
            partial: context.state.partial,
            places: context.state.places,
            visits: context.visits,
        };
        Some(Self { lowerer, plan })
    }
}
