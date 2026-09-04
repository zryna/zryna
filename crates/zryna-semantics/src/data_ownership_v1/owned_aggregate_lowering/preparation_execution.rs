use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::super::Ty;
use super::super::super::owned_constructor_plan::ConstructorKind;
use super::super::super::owned_lowering_resources::{
    CleanupRecipe, CleanupUsage, OwnedCleanupPlanContext,
};
use super::super::PrivateOwnedAggregateLowerer;
use super::super::constructor_resources::ConstructorCommitReservation;
use super::super::preparation_plan::{Leaf, Operation, PreparationPlan, Step};
use super::super::projection_topology::{
    MaterializedProjectionTopology, ProjectionDescriptor, project,
};
use super::PreparedValue;

struct OpenConstructor {
    end: usize,
    ty: Ty,
    kind: ConstructorKind,
    arity: usize,
    values: Vec<raw::ValueId>,
    reservation: Option<ConstructorCommitReservation>,
}

struct Consumption<'l, 'a, 'f, 'e> {
    lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
    open: Vec<OpenConstructor>,
    released: Option<OpenConstructor>,
    cleanups: Vec<(raw::CleanupPlanId, Option<raw::PlaceId>)>,
}

impl Consumption<'_, '_, '_, '_> {
    fn execute(&mut self, index: usize, length: usize, step: Step<'_>) -> Option<raw::ValueId> {
        assert!(
            self.released.is_none() || matches!(step.operation, Operation::Commit { .. }),
            "released constructor must commit next"
        );
        let emission = match step.operation {
            Operation::Enter { arity, kind, end } => {
                assert!(self.cleanups.is_empty(), "constructor cannot interrupt cleanup effects");
                assert!(end > index + 1 && end <= length, "invalid constructor range");
                if let Some(parent) = self.open.last() {
                    assert!(end < parent.end, "child range must end before parent");
                }
                let reservation = self
                    .lowerer
                    .reserve_constructor_commit(step.ty, arity, step.at)
                    .expect("prepared constructor capacity");
                self.open.push(OpenConstructor {
                    end,
                    ty: step.ty,
                    kind,
                    arity,
                    values: Vec::with_capacity(arity),
                    reservation: Some(reservation),
                });
                None
            }
            Operation::Release => {
                assert!(
                    self.cleanups.is_empty(),
                    "constructor release cannot interrupt cleanup effects"
                );
                let mut constructor = self.open.pop().expect("constructor release has one ticket");
                assert_eq!(constructor.ty, step.ty, "constructor release type");
                assert_eq!(
                    index.checked_add(2),
                    Some(constructor.end),
                    "constructor release range"
                );
                constructor
                    .reservation
                    .take()
                    .expect("one constructor ticket")
                    .release(self.lowerer);
                self.released = Some(constructor);
                None
            }
            Operation::Prefix { id, descriptor } => {
                assert!(self.cleanups.is_empty(), "projection cannot interrupt cleanup effects");
                self.lowerer.consume_prepared_prefix(id, descriptor);
                None
            }
            Operation::Cleanup { id, actions, prefix } => {
                assert!(self.cleanups.len() < 2, "at most two cleanup events per admitted leaf");
                self.lowerer.consume_prepared_cleanup(id, actions, prefix, step.at);
                self.cleanups.push((id, prefix));
                None
            }
            Operation::Leaf(leaf) => {
                check_cleanup_link(&leaf, &self.cleanups, self.lowerer.places.len());
                self.cleanups.clear();
                Some(
                    self.lowerer
                        .consume_prepared_leaf(leaf, step.ty, step.at)
                        .expect("prepared leaf emission"),
                )
            }
            Operation::Commit { kind, values } => {
                assert!(self.cleanups.is_empty(), "constructor commit cannot consume leaf cleanup");
                let constructor = self.released.take().expect("constructor owns released contract");
                assert_eq!(
                    (constructor.end, constructor.ty, constructor.arity, constructor.kind),
                    (index + 1, step.ty, values.len(), kind),
                    "constructor commit owns exact released contract"
                );
                assert_eq!(
                    constructor.values, values,
                    "constructor operands match ordered immediate child results"
                );
                Some(
                    self.lowerer
                        .commit_constructor(step.ty, kind, &values, step.at)
                        .expect("prepared constructor commit"),
                )
            }
        };
        let value = emission.as_ref().map(|emission| emission.value);
        let owners = emission.as_ref().map_or(&[][..], |emission| emission.owners.as_slice());
        assert_eq!(value, step.value, "prepared value identity");
        assert_eq!(owners, step.owners, "prepared ordered owner effects");
        if let (Some(value), Some(parent)) = (value, self.open.last_mut()) {
            assert!(parent.values.len() < parent.arity, "constructor has no extra child result");
            parent.values.push(value);
        }
        assert_eq!(self.lowerer.preparation_checkpoint(), step.after, "prepared step effects");
        value
    }
}

impl PreparedValue<'_, '_, '_, '_> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn consume(self) -> raw::ValueId {
        let Self { lowerer, mut plan } = self;
        assert_eq!(lowerer.preparation_checkpoint(), plan.start, "prepared value start changed");
        let original_places = lowerer.places.len();
        let mut checkpoint = plan.start;
        let mut visits = 0usize;
        let mut last_result = None;
        let steps_len = plan.steps.len();
        let mut consumption =
            Consumption { lowerer, open: Vec::new(), released: None, cleanups: Vec::new() };
        for (index, step) in std::mem::take(&mut plan.steps).into_iter().enumerate() {
            assert_eq!(
                consumption.lowerer.preparation_checkpoint(),
                checkpoint,
                "prepared step start changed"
            );
            let after = step.after;
            let ty = step.ty;
            if let Some(value) = consumption.execute(index, steps_len, step) {
                visits = visits.checked_add(1).expect("prepared visit count");
                last_result = Some((value, ty));
            }
            checkpoint = after;
        }
        assert!(
            consumption.open.is_empty() && consumption.released.is_none(),
            "all constructor ranges consumed"
        );
        assert!(consumption.cleanups.is_empty(), "all cleanup events consumed");
        assert_final(consumption.lowerer, &plan, original_places, last_result, visits);
        plan.result
    }
}
fn assert_final(
    lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>,
    plan: &PreparationPlan<'_>,
    original_places: usize,
    last_result: Option<(raw::ValueId, Ty)>,
    visits: usize,
) {
    assert_eq!(
        last_result,
        Some((plan.result, plan.result_type)),
        "prepared root result and exact type"
    );
    assert_eq!(visits, plan.visits, "one consumed result per classified expression");
    assert_eq!(lowerer.owners, plan.owners, "prepared owner map and pending order");
    assert_eq!(lowerer.projections, plan.projections, "prepared canonical projection identities");
    assert_eq!(lowerer.moved_projections, plan.moved, "prepared moved projection mask");
    assert_eq!(lowerer.partial_roots, plan.partial, "prepared partial root mask");
    assert_eq!(
        lowerer.places.len() - original_places,
        plan.places.len(),
        "prepared place suffix length"
    );
    for (index, expected) in plan.places.iter().enumerate() {
        let actual = &lowerer.places[original_places + index];
        assert_eq!(actual.id.0 as usize, original_places + index, "dense prepared place");
        assert_eq!(actual.ty, expected.ty.ir, "prepared place type");
        assert_eq!(actual.span, expected.at, "prepared first place span");
        assert_eq!(actual.kind, expected.kind, "prepared place topology");
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn consume_prepared_prefix(&mut self, id: raw::PlaceId, descriptor: ProjectionDescriptor) {
        let reserved_places = self.reserved_constructor_places();
        let actual = project(
            &mut MaterializedProjectionTopology {
                projections: &mut self.projections,
                places: &mut self.places,
                reserved_places,
            },
            descriptor,
            self.errors,
        )
        .expect("prepared projection capacity");
        assert_eq!(actual, id, "prepared projection identity");
    }

    fn consume_prepared_leaf(
        &mut self,
        leaf: Leaf<'_>,
        ty: Ty,
        at: Span,
    ) -> Option<super::super::state::Emission> {
        match leaf {
            Leaf::Bool(value) => {
                self.emit_recorded(ty, at, raw::InstructionKind::BoolLiteral(value))
            }
            Leaf::I32(value) => self.emit_recorded(ty, at, raw::InstructionKind::I32Literal(value)),
            Leaf::String { bytes, cleanup } => self.emit_recorded(
                ty,
                at,
                raw::InstructionKind::StringFromUtf8 { bytes: bytes.to_vec(), cleanup },
            ),
            Leaf::Reference(decision) => self.emit_reference_recorded(decision, ty, at),
            Leaf::Projection { source, operation } => {
                self.emit_projection_recorded(source, ty, at, &operation)
            }
            Leaf::StringClone { source, cleanup } => {
                self.emit_prepared_string_clone(source, ty, at, cleanup)
            }
            Leaf::AggregateClone { source, cleanup, prefix } => {
                self.emit_prepared_aggregate_clone(source, ty, at, cleanup, prefix)
            }
        }
    }

    fn consume_prepared_cleanup(
        &mut self,
        id: raw::CleanupPlanId,
        actions: usize,
        prefix: Option<raw::PlaceId>,
        at: Span,
    ) {
        let usage = CleanupUsage {
            plans: self.cleanup_plans.len(),
            actions: self.cleanup_actions,
            reserved_plans: 0,
            reserved_actions: 0,
        };
        let actual = match prefix {
            None => self.push_cleanup(at, None),
            Some(owner) => self.push_aggregate_clone_prefix_cleanup(at, owner),
        }
        .expect("prepared cleanup emission");
        assert_eq!(actual, id, "prepared cleanup identity");
        // The owner state has not changed across this event. Expand the same recipe only
        // after real emission, so comparison is linear in the actually charged actions.
        let recipe = match prefix {
            None => CleanupRecipe::reverse(
                &usage,
                self.owners.pending(),
                None,
                OwnedCleanupPlanContext::Aggregate,
                at,
                self.errors,
            ),
            Some(owner) => {
                CleanupRecipe::aggregate_prefix(usage.plans, self.owners.pending(), owner)
            }
        }
        .expect("prepared cleanup recipe");
        assert_eq!(recipe.action_count, actions, "prepared cleanup action count");
        let actual = &self.cleanup_plans[id.0 as usize];
        assert_eq!(actual.id, id);
        assert!(
            actual.actions.iter().copied().eq(recipe.into_actions()),
            "prepared cleanup action order"
        );
    }
}

fn check_cleanup_link(
    leaf: &Leaf<'_>,
    events: &[(raw::CleanupPlanId, Option<raw::PlaceId>)],
    places: usize,
) {
    match leaf {
        Leaf::String { cleanup, .. } | Leaf::StringClone { cleanup, .. } => {
            assert_eq!(events, &[(*cleanup, None)], "fallible leaf cleanup linkage");
        }
        Leaf::AggregateClone { cleanup, prefix, .. } => {
            let owner = raw::PlaceId(u32::try_from(places).expect("prepared clone owner identity"));
            assert_eq!(
                events,
                &[(*cleanup, None), (*prefix, Some(owner))],
                "clone cleanup role linkage"
            );
        }
        _ => assert!(events.is_empty(), "infallible leaf has no cleanup events"),
    }
}
