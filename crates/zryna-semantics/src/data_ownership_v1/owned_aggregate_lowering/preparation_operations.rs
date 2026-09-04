use std::collections::BTreeMap;
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::super::owned_constructor_plan::{ConstructorKind, ConstructorShape};
use super::super::type_model::OwnedAggregatePlace;
use super::super::{Binding, Ty};
use super::availability::AvailabilityView;
use super::expression_decisions::ExpressionDecisions;
use super::operand_decisions::{OperandDecisions, ProjectionOperation, ReferenceKind};
use super::preparation_plan::{Leaf, Operation, Step};
use super::preparation_state::{PreparationState, PreparationTopology};
use super::projection_resolution::ProjectionResolver;
use super::state::constructor_result;

pub(super) struct PreparationContext<'a, 'f, 's, 'e> {
    pub(super) decisions: ExpressionDecisions<'a, 'f, 'e>,
    pub(super) catalog: &'a super::super::function_catalog::FunctionCatalog,
    pub(super) bindings: &'s BTreeMap<String, Binding>,
    pub(super) state: PreparationState<'s>,
    pub(super) aggregate_subobject_moves: usize,
    pub(super) steps: Vec<Step<'f>>,
    pub(super) visits: usize,
}

impl<'a, 'f> PreparationContext<'a, 'f, '_, '_> {
    fn operands(
        &mut self,
    ) -> OperandDecisions<'a, 'f, '_, impl Fn(raw::PlaceId) -> Option<raw::PlaceId> + '_> {
        let state = &self.state;
        OperandDecisions {
            input: self.decisions.input,
            function: self.decisions.function,
            bindings: self.bindings,
            layouts: self.decisions.layouts,
            availability: AvailabilityView::new(
                &state.owners,
                &state.moved,
                &state.partial,
                |id| state.parent(id),
            ),
            aggregate_subobject_moves: self.aggregate_subobject_moves,
            errors: self.decisions.errors,
        }
    }

    pub(super) fn push(
        &mut self,
        operation: Operation<'f>,
        ty: Ty,
        at: Span,
        value: Option<raw::ValueId>,
    ) {
        self.steps.push(Step {
            operation,
            ty,
            at,
            value,
            owners: Vec::new(),
            after: self.state.checkpoint(),
        });
    }

    pub(super) fn reverse(&mut self, ty: Ty, at: Span) -> Option<raw::CleanupPlanId> {
        let (id, actions) = self.state.reverse_cleanup(at, self.decisions.errors)?;
        self.push(Operation::Cleanup { id, actions, prefix: None }, ty, at, None);
        Some(id)
    }

    pub(super) fn emit_leaf(&mut self, leaf: Leaf<'f>, ty: Ty, at: Span) -> Option<raw::ValueId> {
        let mut emission = self.state.emit(ty, at, self.decisions.errors)?;
        match &leaf {
            Leaf::Reference(decision) if matches!(decision.kind, ReferenceKind::Move) => {
                emission.owners.push(
                    self.state.owners.rehome_move_result(emission.value, decision.binding.place)?,
                );
            }
            Leaf::Projection {
                source,
                operation: ProjectionOperation::Move { aggregate_subobject },
            } => {
                assert!(
                    !aggregate_subobject,
                    "constructor child cannot acquire contextual aggregate move authority"
                );
                self.state.moved.insert(source.place);
                self.state.partial.insert(source.root);
            }
            _ => {}
        }
        for delta in &emission.owners {
            super::super::owner_state::apply_owner_delta(
                &mut self.state.facts.string_bytes,
                *delta,
            );
        }
        if let Some(owner) = self.state.owners.owner(emission.value) {
            let bytes = match &leaf {
                Leaf::String { bytes, .. } => Some(u64::try_from(bytes.len()).ok()?),
                Leaf::StringClone { bytes, .. } | Leaf::StringConcat { bytes, .. } => bytes.known(),
                _ => None,
            };
            if let Some(bytes) = bytes {
                self.state.facts.string_bytes.insert(owner, bytes);
            }
        }
        let value = emission.value;
        self.steps.push(Step {
            operation: Operation::Leaf(leaf),
            ty,
            at,
            value: Some(value),
            owners: emission.owners,
            after: self.state.checkpoint(),
        });
        Some(value)
    }

    pub(super) fn reference(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let decision = self.operands().reference_decision(name, ty)?;
        self.emit_leaf(Leaf::Reference(decision), ty, at)
    }

    pub(super) fn resolve(&mut self, id: u32) -> Option<OwnedAggregatePlace> {
        let mut inserted = Vec::new();
        let decisions = &mut self.decisions;
        let result = ProjectionResolver {
            input: decisions.input,
            file: decisions.file,
            function: decisions.function,
            module: decisions.module,
            declarations: decisions.declarations,
            graph: decisions.graph,
            node_types: decisions.node_types,
            layouts: decisions.layouts,
            bindings: self.bindings,
            errors: decisions.errors,
        }
        .resolve(id, &mut PreparationTopology { state: &mut self.state, inserted: &mut inserted });
        for (id, descriptor, after) in inserted {
            self.steps.push(Step {
                ty: descriptor.ty,
                at: descriptor.at,
                operation: Operation::Prefix { id, descriptor },
                value: None,
                owners: Vec::new(),
                after,
            });
        }
        result
    }

    pub(super) fn projection(&mut self, id: u32, ty: Ty, at: Span) -> Option<raw::ValueId> {
        let source = self.resolve(id)?;
        let operation = self.operands().projection_decision(source, ty, None, at)?;
        self.emit_leaf(Leaf::Projection { source, operation }, ty, at)
    }

    pub(super) fn string_read_projection(
        &mut self,
        id: u32,
        ty: Ty,
        at: Span,
    ) -> Option<OwnedAggregatePlace> {
        let source = self.resolve(id)?;
        self.operands().string_clone_source(source, ty, at)
    }

    pub(super) fn string_clone(&mut self, id: u32, ty: Ty, at: Span) -> Option<raw::ValueId> {
        if self.state.summary
            && let syntax::RawExpressionKind::Reference { name } =
                &self.decisions.function.body.expressions.get(id as usize)?.kind
        {
            let (place, bytes) = super::super::owned_string_read::local_source(
                name,
                self.bindings,
                &self.state.owners,
                &self.state.facts.string_bytes,
                Some(ty),
                super::super::span(self.decisions.input.sources(), name.span),
                self.decisions.errors,
            )?;
            let source =
                OwnedAggregatePlace { ty, place, root: place, mutable: false, is_root: true };
            let cleanup = self.reverse(ty, at)?;
            return self.emit_leaf(Leaf::StringClone { source, bytes, cleanup }, ty, at);
        }
        let source = self.resolve(id)?;
        let usage = self.state.clone_usage();
        let source = if self.state.summary {
            let source = self.operands().string_clone_source(source, ty, at)?;
            self.push(Operation::CloneCapacity { aggregate: false }, ty, at, None);
            source
        } else {
            self.operands().string_clone_decision(source, ty, at, &usage)?
        };
        let cleanup = self.reverse(ty, at)?;
        let bytes = super::super::owned_string_read::StringBytes::from_known(
            self.state.facts.string_bytes.get(&source.place).copied(),
        );
        self.emit_leaf(Leaf::StringClone { source, bytes, cleanup }, ty, at)
    }

    pub(super) fn aggregate_clone(&mut self, id: u32, ty: Ty, at: Span) -> Option<raw::ValueId> {
        let usage = self.state.clone_usage();
        let binding = if self.state.summary {
            let binding = self.operands().aggregate_clone_source(id, ty, at)?;
            self.push(Operation::CloneCapacity { aggregate: true }, ty, at, None);
            binding
        } else {
            self.operands().aggregate_clone_decision(id, ty, at, &usage)?
        };
        let cleanup = self.reverse(ty, at)?;
        let owner = raw::PlaceId(u32::try_from(self.state.counts[1]).ok()?);
        let (prefix, actions) = self.state.prefix_cleanup(owner)?;
        self.push(Operation::Cleanup { id: prefix, actions, prefix: Some(owner) }, ty, at, None);
        self.emit_leaf(Leaf::AggregateClone { source: binding.place, cleanup, prefix }, ty, at)
    }

    pub(super) fn commit(
        &mut self,
        ty: Ty,
        at: Span,
        kind: ConstructorKind,
        values: Vec<raw::ValueId>,
        cleanup: Option<raw::CleanupPlanId>,
    ) -> Option<raw::ValueId> {
        if !self.state.summary
            && !self.state.usage().operands(values.len(), at, self.decisions.errors)
        {
            return None;
        }
        self.state.counts[3] = self.state.counts[3].checked_add(values.len())?;
        let prepared =
            ConstructorShape::derive(self.decisions.layouts, ty, kind, values.len(), |id| {
                self.decisions.node_types.iter().flatten().find(|ty| ty.layout == id).copied()
            })
            .and_then(|shape| {
                let types = self.state.types.as_ref().map_err(|error| *error)?;
                self.state.cache = types.checkpoint();
                shape.prepare(&values, |value| types.get(value), &self.state.owners)
            });
        let prepared = constructor_result(prepared, at, self.decisions.errors)?;
        let mut emission = self.state.emit(prepared.result_type(), at, self.decisions.errors)?;
        emission.owners.extend(prepared.commit(&mut self.state.owners));
        for delta in &emission.owners {
            super::super::owner_state::apply_owner_delta(
                &mut self.state.facts.string_bytes,
                *delta,
            );
        }
        let value = emission.value;
        self.steps.push(Step {
            operation: match cleanup {
                Some(cleanup) => Operation::VecCommit { values, cleanup },
                None => Operation::Commit { kind, values },
            },
            ty,
            at,
            value: Some(value),
            owners: emission.owners,
            after: self.state.checkpoint(),
        });
        Some(value)
    }
}
