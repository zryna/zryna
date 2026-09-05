use super::super::expression_decisions::ExpressionDecisions;
use super::super::mixed_shape;
use super::super::preparation_state::PreparationState;
use super::{
    PreparationContext, PreparationPlan, PreparedValue, PrivateOwnedAggregateLowerer, Ty,
    resource_replay,
};
use crate::data_ownership_v1::diagnostics::span;

#[derive(Clone, Copy)]
enum PreparationSite {
    RootTopology,
    LocalInitializer,
    Replacement { target: super::raw::PlaceId },
}

impl<'l, 'a, 'f, 'e> PreparedValue<'l, 'a, 'f, 'e> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn prepare(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
    ) -> Option<Self> {
        Self::prepare_at(lowerer, id, expected, PreparationSite::RootTopology)
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn prepare_local(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
    ) -> Option<Self> {
        Self::prepare_at(lowerer, id, expected, PreparationSite::LocalInitializer)
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn prepare_replacement(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
        target: super::raw::PlaceId,
    ) -> Option<Self> {
        assert!(lowerer.mixed_replacement_target(expected), "mixed replacement target type");
        Self::prepare_at(lowerer, id, expected, PreparationSite::Replacement { target })
    }

    fn prepare_at(
        lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
        id: u32,
        expected: Ty,
        site: PreparationSite,
    ) -> Option<Self> {
        let start = lowerer.preparation_checkpoint();
        let route = match site {
            PreparationSite::RootTopology | PreparationSite::Replacement { .. } => {
                mixed_shape::route(expected, lowerer.layouts)
            }
            PreparationSite::LocalInitializer => lowerer.local_preparation_route(expected),
        };
        if route == mixed_shape::PreparationRoute::LegacyVec {
            lowerer.errors.at(
                "ZRYNA-M3016",
                span(
                    lowerer.input.sources(),
                    lowerer.function.body.expressions.get(id as usize)?.span,
                ),
                "scalar and String Vec roots require their existing ordered lowering route",
                "keep this Vec root on its established construction authority",
            );
            return None;
        }
        let summary = route == mixed_shape::PreparationRoute::MixedSummary;
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
        if let PreparationSite::Replacement { target } = site {
            let mut owners = plan.owners.clone();
            if plan.partial.contains(&target) || owners.replace(result, target).is_none() {
                lowerer.errors.at(
                    "ZRYNA-M3014",
                    span(lowerer.input.sources(), lowerer.function.body.expressions.get(id as usize)?.span),
                    "owned aggregate assignment cannot consume its destination while preparing its replacement",
                    "clone the destination or prepare a distinct aggregate value before replacement",
                );
                return None;
            }
        }
        if summary {
            resource_replay::validate(&mut plan, lowerer.layouts, lowerer.errors)?;
        }
        Some(Self { lowerer, plan })
    }
}
