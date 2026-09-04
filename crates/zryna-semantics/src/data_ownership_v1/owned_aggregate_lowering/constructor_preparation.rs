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
use super::preparation_plan::{StringOperation, StringRead};
use super::preparation_state::PreparationState;

#[cfg(test)]
#[path = "../tests/mixed_byte_facts.rs"]
mod byte_facts;
#[cfg(test)]
#[path = "../tests/mixed_call_consumption_misuse.rs"]
mod call_misuse;
#[cfg(test)]
#[path = "../tests/mixed_call_resource_controls.rs"]
mod call_resource_controls;
#[cfg(test)]
#[path = "../tests/mixed_call_resource_order.rs"]
mod call_resource_order;
#[path = "preparation_call_resources.rs"]
mod call_resources;
#[path = "preparation_call_scope.rs"]
mod call_scope;
#[cfg(test)]
#[path = "../tests/mixed_cleanup_frontiers.rs"]
mod cleanup_frontiers;
#[cfg(test)]
#[path = "../tests/contextual_local_routing.rs"]
mod contextual_local_routing;
#[path = "preparation_execution.rs"]
mod execution;
#[cfg(test)]
#[path = "../tests/mixed_optional_string_bytes.rs"]
mod optional_string_bytes;
#[cfg(test)]
#[path = "../tests/mixed_phase_controls.rs"]
mod phase_controls;
#[path = "preparation_resource_replay.rs"]
mod resource_replay;
#[path = "preparation_scalar_scope.rs"]
mod scalar_scope;
#[cfg(test)]
#[path = "../tests/mixed_string_call_facts.rs"]
mod string_call_facts;
#[cfg(test)]
#[path = "../tests/mixed_string_read_boundaries.rs"]
mod string_read_boundaries;
#[cfg(test)]
#[path = "../tests/mixed_string_read_facts.rs"]
mod string_read_facts;
#[cfg(test)]
#[path = "../tests/mixed_string_read_misuse.rs"]
mod string_read_misuse;
#[path = "preparation_string_scope.rs"]
mod string_scope;
#[cfg(test)]
#[path = "../tests/mixed_unknown_projected_controls.rs"]
mod unknown_projected_controls;
#[cfg(test)]
#[path = "../tests/mixed_vec_sibling_controls.rs"]
mod vec_sibling_controls;

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
    Call(call_scope::CallFrame),
    Visit(u32, Option<Ty>),
    Scalar(scalar_scope::ScalarFrame),
    Constructor(ConstructorFrame<'f>),
    String(StringFrame),
    Read(u32, Ty),
    ReadResult(Ty, Span),
}

enum VisitOutcome {
    Value(raw::ValueId),
    Deferred,
}

struct StringFrame {
    kind: StringOperation,
    inputs: Vec<u32>,
    reads: Vec<StringRead>,
    ty: Ty,
    at: Span,
    start: usize,
    next: usize,
    waiting: bool,
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
        if !self.state.summary
            && !self.state.usage().constructor(ty, arity, at, self.decisions.errors)
        {
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

    fn expression_span(&self, id: u32) -> Option<Span> {
        let expression = self.decisions.function.body.expressions.get(usize::try_from(id).ok()?)?;
        Some(super::super::span(self.decisions.input.sources(), expression.span))
    }

    fn visit(
        &mut self,
        id: u32,
        expected: Option<Ty>,
        frames: &mut Vec<Frame<'f>>,
    ) -> Option<VisitOutcome> {
        self.visits = self.visits.checked_add(1)?;
        let decision = self.decisions.classify_prepared(id, expected, self.state.summary)?;
        let at = decision.at;
        if let ExpressionKind::Scalar { operation, ref inputs } = decision.kind {
            frames.push(self.enter_scalar(operation, inputs.clone(), expected, decision.ty?, at));
            return Some(VisitOutcome::Deferred);
        }
        if let ExpressionKind::Call { callee, arguments } = decision.kind {
            frames.push(self.enter_call(callee, arguments, decision.ty, at)?);
            return Some(VisitOutcome::Deferred);
        }
        if let ExpressionKind::InferredClone(operand) = decision.kind {
            return self.inferred_clone(operand, at, frames);
        }
        let ty = match (&decision.kind, decision.ty) {
            (_, Some(ty)) => ty,
            (ExpressionKind::Reference(name), None) => self.inferred_reference_type(name)?,
            (ExpressionKind::Projection(id), None) => {
                let source = self.resolve(*id)?;
                let value = self.resolved_projection(source, source.ty, at)?;
                return Some(VisitOutcome::Value(value));
            }
            _ => {
                self.decisions.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "expression requires an exact owned contextual type",
                    "use a supported typed scalar operand",
                );
                return None;
            }
        };
        let value = match decision.kind {
            ExpressionKind::Scalar { .. } => unreachable!("scalar frame entered"),
            ExpressionKind::InferredClone(_) => unreachable!("inferred clone selected"),
            ExpressionKind::Bool(value) => self.emit_leaf(Leaf::Bool(value), ty, at),
            ExpressionKind::I32(value) => self.emit_leaf(Leaf::I32(value), ty, at),
            ExpressionKind::String(bytes) => {
                let cleanup = self.reverse(ty, at)?;
                self.emit_leaf(Leaf::String { bytes, cleanup }, ty, at)
            }
            ExpressionKind::Reference(name) => self.reference(name, ty, at),
            ExpressionKind::Projection(id) => self.projection(id, ty, at),
            ExpressionKind::StringClone(id) => {
                if self.state.summary && self.compound_string_read(id)? {
                    frames.push(self.enter_string(StringOperation::Clone, vec![id], ty, at)?);
                    return Some(VisitOutcome::Deferred);
                }
                self.string_clone(id, ty, at)
            }
            ExpressionKind::StringConcat { arguments, callee } => {
                self.require_string_scope(at)?;
                let inputs = super::super::owned_string_read::concat_arguments(
                    arguments,
                    callee,
                    self.decisions.errors,
                )?;
                frames.push(self.enter_string(StringOperation::Concat, inputs.to_vec(), ty, at)?);
                return Some(VisitOutcome::Deferred);
            }
            ExpressionKind::AggregateClone(id) => self.aggregate_clone(id, ty, at),
            ExpressionKind::Call { .. } => unreachable!("call frame entered"),
            ExpressionKind::Struct(decision) => {
                frames.push(self.enter(
                    Children::Struct(decision),
                    ty,
                    at,
                    ConstructorKind::Struct,
                )?);
                return Some(VisitOutcome::Deferred);
            }
            ExpressionKind::Array(decision) => {
                frames.push(self.enter(
                    Children::Array(decision),
                    ty,
                    at,
                    ConstructorKind::FixedArray,
                )?);
                return Some(VisitOutcome::Deferred);
            }
            ExpressionKind::Vec(decision) => {
                frames.push(self.enter(Children::Array(decision), ty, at, ConstructorKind::Vec)?);
                return Some(VisitOutcome::Deferred);
            }
            ExpressionKind::Enum(decision) => {
                let kind = ConstructorKind::Enum { variant: u32::try_from(decision.ordinal).ok()? };
                frames.push(self.enter(Children::Enum(decision.payload_input), ty, at, kind)?);
                return Some(VisitOutcome::Deferred);
            }
        };
        Some(VisitOutcome::Value(value?))
    }

    fn walk(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let mut frames = vec![Frame::Visit(id, Some(expected))];
        let mut result = None;
        let mut read_result = None;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Scalar(frame) => {
                    if let VisitOutcome::Value(value) =
                        self.advance_scalar(frame, &mut result, &mut frames)?
                    {
                        result = Some(value);
                    }
                }
                Frame::Call(mut frame) => {
                    if frame.waiting {
                        frame.values.push(result.take()?);
                    }
                    if let Some(&id) = frame.inputs.get(frame.next) {
                        let ty = frame.signature.parameter?;
                        frame.next += 1;
                        frame.waiting = true;
                        frames.push(Frame::Call(frame));
                        frames.push(Frame::Visit(id, Some(ty)));
                    } else {
                        result = Some(self.finish_call(frame)?);
                    }
                }
                Frame::Visit(id, ty) => {
                    if let VisitOutcome::Value(value) = self.visit(id, ty, &mut frames)? {
                        result = Some(value);
                    }
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
                        frames.push(Frame::Visit(id, Some(ty)));
                    } else {
                        result = Some(self.finish(frame)?);
                    }
                }
                Frame::String(mut frame) => {
                    if frame.waiting {
                        frame.reads.push(read_result.take()?);
                    }
                    if let Some(&id) = frame.inputs.get(frame.next) {
                        frame.next += 1;
                        frame.waiting = true;
                        let ty = frame.ty;
                        frames.push(Frame::String(frame));
                        frames.push(Frame::Read(id, ty));
                    } else {
                        result = Some(self.finish_string(frame)?);
                    }
                }
                Frame::Read(id, ty) => {
                    if let string_scope::ReadSelection::Place(read) =
                        self.read_local_string(id, ty)?
                    {
                        read_result = Some(read);
                    } else {
                        let at = self.expression_span(id)?;
                        frames.push(Frame::ReadResult(ty, at));
                        frames.push(Frame::Visit(id, Some(ty)));
                    }
                }
                Frame::ReadResult(ty, at) => {
                    read_result = Some(self.read_result(result.take()?, ty, at)?);
                }
            }
        }
        result
    }
    fn finish(&mut self, frame: ConstructorFrame<'f>) -> Option<raw::ValueId> {
        self.state.ledger().release_constructor(frame.reservation);
        self.push(Operation::Release, frame.ty, frame.at, None);
        let cleanup = if frame.kind == ConstructorKind::Vec {
            Some(self.reverse(frame.ty, frame.at)?)
        } else {
            None
        };
        let value = self.commit(frame.ty, frame.at, frame.kind, frame.values, cleanup)?;
        let end = self.steps.len();
        let Operation::Enter { end: slot, .. } = &mut self.steps[frame.start].operation else {
            unreachable!("constructor frame retains its own entry");
        };
        *slot = end;
        Some(value)
    }
}

// The exclusive borrow binds preparation and consumption to one real lowerer state.
// Rejection drops scratch metadata only; no rollback of real arenas or cache is needed.
pub(super) struct PreparedValue<'l, 'a, 'f, 'e> {
    lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
    plan: PreparationPlan<'f>,
}

#[derive(Clone, Copy)]
enum PreparationSite {
    RootTopology,
    LocalInitializer,
}

impl<'l, 'a, 'f, 'e> PreparedValue<'l, 'a, 'f, 'e> {
    pub(super) fn prepare(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
    ) -> Option<Self> {
        Self::prepare_at(lowerer, id, expected, PreparationSite::RootTopology)
    }

    pub(super) fn prepare_local(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
    ) -> Option<Self> {
        Self::prepare_at(lowerer, id, expected, PreparationSite::LocalInitializer)
    }

    fn prepare_at(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
        site: PreparationSite,
    ) -> Option<Self> {
        let start = lowerer.preparation_checkpoint();
        let route = match site {
            PreparationSite::RootTopology => super::mixed_shape::route(expected, lowerer.layouts),
            PreparationSite::LocalInitializer => lowerer.local_preparation_route(expected),
        };
        if route == super::mixed_shape::PreparationRoute::LegacyVec {
            lowerer.errors.at(
                "ZRYNA-M3016",
                super::super::span(
                    lowerer.input.sources(),
                    lowerer.function.body.expressions.get(id as usize)?.span,
                ),
                "scalar and String Vec roots require their existing ordered lowering route",
                "keep this Vec root on its established construction authority",
            );
            return None;
        }
        let summary = route == super::mixed_shape::PreparationRoute::MixedSummary;
        let storage = lowerer.preparation_storage();
        let mut context = PreparationContext {
            catalog: lowerer.catalog,
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
                summary,
                facts: lowerer.preparation_facts.clone(),
            },
            aggregate_subobject_moves: lowerer.aggregate_subobject_moves,
            steps: Vec::new(),
            visits: 0,
        };
        let result = context.walk(id, expected)?;
        let mut plan = PreparationPlan {
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
            facts: context.state.facts,
        };
        if summary {
            resource_replay::validate(&mut plan, lowerer.layouts, lowerer.errors)?;
        }
        Some(Self { lowerer, plan })
    }
}
#[cfg(test)]
#[path = "../tests/mixed_disjoint_owned_sibling_controls.rs"]
mod mixed_disjoint_owned_sibling_controls;
#[cfg(test)]
#[path = "../tests/scalar_private_controls.rs"]
mod scalar_private_controls;
#[cfg(test)]
#[path = "../tests/scalar_resource_controls.rs"]
mod scalar_resource_controls;
