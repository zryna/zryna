use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::super::Ty;
use super::super::super::owned_constructor_plan::ConstructorKind;
use super::super::super::owned_lowering_resources::{
    CleanupRecipe, CleanupUsage, OwnedCleanupPlanContext, OwnedCleanupReservationContext,
};
use super::super::PrivateOwnedAggregateLowerer;
use super::super::constructor_resources::ConstructorCommitReservation;
use super::super::preparation_plan::{Leaf, Operation, PreparationPlan, Step};
use super::super::projection_topology::{
    MaterializedProjectionTopology, ProjectionDescriptor, project,
};
use super::PreparedValue;
#[path = "preparation_leaf_consumption.rs"]
mod leaf_consumption;
use leaf_consumption::{assert_final, check_cleanup_link};

#[path = "preparation_string_consumption.rs"]
mod string_consumption;
use string_consumption::StringScopes;
#[path = "preparation_call_consumption.rs"]
mod call_consumption;
use call_consumption::CallScopes;

struct OpenConstructor {
    end: usize,
    ty: Ty,
    kind: ConstructorKind,
    arity: usize,
    values: Vec<raw::ValueId>,
    reservation: Option<ConstructorCommitReservation>,
    cleanup_actions: Option<usize>,
}

struct Consumption<'l, 'a, 'f, 'e> {
    lowerer: &'l mut PrivateOwnedAggregateLowerer<'a, 'f, 'e>,
    open: Vec<OpenConstructor>,
    released: Option<OpenConstructor>,
    cleanups: Vec<(raw::CleanupPlanId, Option<raw::PlaceId>)>,
    strings: StringScopes,
    calls: CallScopes,
}

impl Consumption<'_, '_, '_, '_> {
    fn string_step(&mut self, index: usize, length: usize, ty: Ty, operation: Operation<'_>) {
        assert!(self.cleanups.is_empty(), "String operation cannot interrupt cleanup");
        match operation {
            Operation::StringEnter { kind, end, reads } => {
                self.strings.enter((index, end, length), self.open.len(), ty, kind, reads);
            }
            Operation::StringRead(read) => {
                let source =
                    self.lowerer.places.get(read.place.0 as usize).expect("String read place");
                assert_eq!(source.ty, ty.ir, "String read actual exact type");
                assert_eq!(ty.category, zryna_layout::TypeCategory::String, "String read category");
                let availability = super::super::availability::materialized_availability(
                    &self.lowerer.owners,
                    &self.lowerer.moved_projections,
                    &self.lowerer.partial_roots,
                    &self.lowerer.places,
                );
                assert!(
                    availability.place_is_at_or_below(read.place, read.root)
                        && availability.projection_available(read.place, read.root),
                    "String read actual availability and root"
                );
                assert_eq!(
                    super::super::super::owned_string_read::StringBytes::from_known(
                        self.lowerer.preparation_facts.string_bytes.get(&read.place).copied()
                    ),
                    read.bytes,
                    "String read actual byte fact"
                );
                if let Some(value) = read.value {
                    assert_eq!(
                        self.lowerer.owners.owner(value),
                        Some(read.place),
                        "String read actual produced owner"
                    );
                }
                self.strings.read(read, ty);
            }
            Operation::StringExit => self.strings.exit(index, ty, self.open.len()),
            _ => unreachable!("String scope operation"),
        }
    }

    fn enter(
        &mut self,
        index: usize,
        length: usize,
        (ty, at): (Ty, Span),
        (arity, kind, end): (usize, ConstructorKind, usize),
        vec_actions: Option<usize>,
    ) {
        assert!(self.cleanups.is_empty(), "constructor cannot interrupt cleanup effects");
        assert!(end > index + 1 && end <= length, "invalid constructor range");
        if let Some(parent) = self.open.last() {
            assert!(end < parent.end, "child range must end before parent");
        }
        let reservation = self
            .lowerer
            .reserve_constructor_commit(ty, arity, at)
            .expect("prepared constructor capacity");
        if let Some(actions) = vec_actions {
            self.lowerer.preparation_facts.held_cleanup = CleanupUsage {
                plans: self.lowerer.cleanup_plans.len(),
                actions: self.lowerer.cleanup_actions,
                reserved_plans: self.lowerer.preparation_facts.held_cleanup[0],
                reserved_actions: self.lowerer.preparation_facts.held_cleanup[1],
            }
            .reserve(actions, OwnedCleanupReservationContext::Vec, at, self.lowerer.errors)
            .expect("prepared Vec cleanup reservation");
        }
        self.open.push(OpenConstructor {
            end,
            ty,
            kind,
            arity,
            values: Vec::with_capacity(arity),
            reservation: Some(reservation),
            cleanup_actions: vec_actions,
        });
    }

    fn release(&mut self, index: usize, ty: Ty) {
        assert!(self.cleanups.is_empty(), "constructor release cannot interrupt cleanup effects");
        let mut constructor = self.open.pop().expect("constructor release has one ticket");
        assert_eq!(constructor.ty, ty, "constructor release type");
        assert_eq!(
            index.checked_add(if constructor.kind == ConstructorKind::Vec { 3 } else { 2 }),
            Some(constructor.end),
            "constructor release range"
        );
        if let Some(actions) = constructor.cleanup_actions {
            self.lowerer.preparation_facts.held_cleanup =
                CleanupUsage::release(self.lowerer.preparation_facts.held_cleanup, actions);
        }
        constructor.reservation.take().expect("one constructor ticket").release(self.lowerer);
        self.released = Some(constructor);
    }

    fn clone_capacity(&mut self, aggregate: bool, at: Span) {
        assert!(self.cleanups.is_empty(), "clone preflight precedes cleanup events");
        super::resource_replay::clone_capacity(
            self.lowerer.preparation_checkpoint(),
            aggregate,
            at,
            self.lowerer.errors,
        )
        .expect("prepared clone capacity");
    }

    fn require_next(&self, operation: &Operation<'_>) {
        assert!(
            !self.calls.pending()
                || matches!(operation, Operation::Cleanup { .. } | Operation::CallCommit { .. }),
            "call release must be followed by cleanup and result"
        );
        assert!(
            !self.strings.pending_result()
                || matches!(operation, Operation::Cleanup { .. } | Operation::Leaf(_)),
            "String exit must be followed by cleanup and result"
        );
        assert!(
            self.released.is_none()
                || matches!(operation, Operation::Commit { .. } | Operation::VecCommit { .. })
                || (self.released.as_ref().is_some_and(|value| value.kind == ConstructorKind::Vec)
                    && matches!(operation, Operation::Cleanup { .. })),
            "released constructor must commit next"
        );
    }

    fn commit_step(
        &mut self,
        index: usize,
        (ty, at): (Ty, Span),
        operation: Operation<'_>,
    ) -> super::super::state::Emission {
        match operation {
            Operation::Commit { kind, values } => {
                assert!(self.cleanups.is_empty(), "constructor commit cannot consume leaf cleanup");
                let constructor = self.released.take().expect("constructor owns released contract");
                assert_eq!(
                    (constructor.end, constructor.ty, constructor.arity, constructor.kind),
                    (index + 1, ty, values.len(), kind),
                    "constructor commit owns exact released contract"
                );
                assert_eq!(
                    constructor.values, values,
                    "constructor operands match ordered immediate child results"
                );
                self.lowerer
                    .commit_constructor(ty, kind, &values, at)
                    .expect("prepared constructor commit")
            }
            Operation::VecCommit { values, cleanup } => {
                assert_eq!(self.cleanups, [(cleanup, None)], "Vec constructor cleanup linkage");
                self.cleanups.clear();
                let constructor = self.released.take().expect("Vec owns released contract");
                assert_eq!(
                    (constructor.end, constructor.ty, constructor.arity, constructor.kind),
                    (index + 1, ty, values.len(), ConstructorKind::Vec),
                    "Vec exact released contract"
                );
                assert_eq!(
                    constructor.values, values,
                    "constructor operands match ordered immediate child results"
                );
                self.lowerer
                    .commit_constructor_with_cleanup(
                        ty,
                        ConstructorKind::Vec,
                        &values,
                        at,
                        Some(cleanup),
                    )
                    .expect("prepared Vec commit")
            }
            _ => unreachable!("constructor commit operation"),
        }
    }

    fn execute(
        &mut self,
        index: usize,
        length: usize,
        step: Step<'_>,
        vec_actions: Option<usize>,
    ) -> Option<raw::ValueId> {
        self.require_next(&step.operation);
        let mut effects = Vec::new();
        let emission = match step.operation {
            Operation::CallEnter { signature, end, arguments } => {
                self.enter_call(
                    (index, end, length),
                    signature,
                    arguments,
                    (step.ty, step.at),
                    vec_actions.expect("call cleanup demand"),
                );
                None
            }
            Operation::CallTransfer { value, owner } => {
                effects.push(self.transfer_call(index, value, owner, step.ty));
                None
            }
            Operation::CallRelease => {
                self.release_call(index, step.ty);
                None
            }
            Operation::CallCommit { signature, arguments, cleanup } => {
                Some(self.commit_call(index, signature, arguments, cleanup, (step.ty, step.at)))
            }
            operation @ (Operation::StringEnter { .. }
            | Operation::StringRead(_)
            | Operation::StringExit) => {
                self.string_step(index, length, step.ty, operation);
                None
            }
            Operation::Enter { arity, kind, end } => {
                self.enter(index, length, (step.ty, step.at), (arity, kind, end), vec_actions);
                None
            }
            Operation::Release => {
                self.release(index, step.ty);
                None
            }
            Operation::Prefix { id, descriptor } => {
                assert!(self.cleanups.is_empty(), "projection cannot interrupt cleanup effects");
                self.lowerer.consume_prepared_prefix(id, descriptor);
                None
            }
            Operation::CloneCapacity { aggregate } => {
                self.clone_capacity(aggregate, step.at);
                None
            }
            Operation::Cleanup { id, actions, prefix } => {
                assert!(self.cleanups.len() < 2, "at most two cleanup events per admitted leaf");
                self.lowerer.consume_prepared_cleanup(id, actions, prefix, step.at);
                self.cleanups.push((id, prefix));
                None
            }
            Operation::Leaf(leaf) => {
                self.strings.leaf(index, step.ty, &leaf);
                check_cleanup_link(&leaf, &self.cleanups, self.lowerer.places.len());
                self.cleanups.clear();
                Some(
                    self.lowerer
                        .consume_prepared_leaf(leaf, step.ty, step.at)
                        .expect("prepared leaf emission"),
                )
            }
            operation @ (Operation::Commit { .. } | Operation::VecCommit { .. }) => {
                Some(self.commit_step(index, (step.ty, step.at), operation))
            }
        };
        let value = emission.as_ref().map(|emission| emission.value);
        let owners =
            emission.as_ref().map_or(effects.as_slice(), |emission| emission.owners.as_slice());
        assert_eq!(value, step.value, "prepared value identity");
        assert_eq!(owners, step.owners, "prepared ordered owner effects");
        let outward = value.is_some_and(|value| {
            if self.calls.start() > self.strings.start() {
                self.calls.result(value, self.open.len())
            } else {
                self.strings.result(value, self.open.len())
            }
        });
        if let (Some(value), Some(parent)) = (value.filter(|_| outward), self.open.last_mut()) {
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
        let mut consumption = Consumption {
            lowerer,
            open: Vec::new(),
            released: None,
            cleanups: Vec::new(),
            strings: StringScopes::default(),
            calls: CallScopes::default(),
        };
        let steps = std::mem::take(&mut plan.steps);
        let vec_actions: Vec<_> = steps
            .iter()
            .map(|step| match step.operation {
                Operation::Enter { kind: ConstructorKind::Vec, end, .. }
                | Operation::CallEnter { end, .. } => {
                    match steps.get(end.checked_sub(2).expect("Vec range")) {
                        Some(Step {
                            operation: Operation::Cleanup { actions, prefix: None, .. },
                            ..
                        }) => Some(*actions),
                        _ => panic!("Vec range ends with reverse cleanup"),
                    }
                }
                _ => None,
            })
            .collect();
        for (index, (step, actions)) in steps.into_iter().zip(vec_actions).enumerate() {
            assert_eq!(
                consumption.lowerer.preparation_checkpoint(),
                checkpoint,
                "prepared step start changed"
            );
            let after = step.after;
            let ty = step.ty;
            if let Some(value) = consumption.execute(index, steps_len, step, actions) {
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
        assert!(consumption.strings.complete(), "all String scopes consumed");
        assert!(consumption.calls.complete(), "all call scopes consumed");
        assert_final(consumption.lowerer, &plan, original_places, last_result, visits);
        plan.result
    }
}
