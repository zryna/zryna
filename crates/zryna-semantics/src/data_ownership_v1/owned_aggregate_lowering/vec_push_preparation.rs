use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::diagnostics::span;
use super::super::type_model::OwnedAggregatePlace;
use super::availability::AvailabilityView;
use super::constructor_preparation::PreparedValue;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;

impl super::PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn lower_generic_vec_push(&mut self, id: u32) -> Option<()> {
        let expression = self.expression(id)?;
        let RawExpressionKind::VecPush { vector, value, .. } = expression.kind else {
            return None;
        };
        let at = span(self.input.sources(), expression.span);
        let Some(vector_ty) =
            self.projection_expression_type(vector).filter(|ty| ty.category == TypeCategory::Vec)
        else {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "push requires an addressable exact Vec target",
                "push into one mutable initialized Vec local or static projection",
            );
            return None;
        };
        let element = self.layouts.type_by_id(vector_ty.layout)?.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        PreparedValue::prepare_push(self, vector, value, ty, at)?.consume();
        Some(())
    }
}

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn vec_push(
        &mut self,
        vector: u32,
        value: u32,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let source = self.resolve(vector)?;
        self.available_push_target(source, at)?;
        let value = self.walk(value, ty)?;
        // RHS effects may have consumed the target through another expression or call.
        self.available_push_target(source, at)?;
        if !ty.is_copy() {
            let owner = self.state.owners.owner(value)?;
            self.check_access(owner, false, at)?;
            if !AvailabilityView::new(
                &self.state.owners,
                &self.state.moved,
                &self.state.partial,
                |id| self.state.parent(id),
            )
            .whole_root_available(owner)
            {
                self.decisions.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "push value is not one complete independently prepared owner",
                    "prepare an exact fully initialized element before appending it",
                );
                return None;
            }
        }
        // Growth failure still owns both the complete argument and the unchanged vector.
        let cleanup = self.reverse(ty, at)?;
        let delta = if ty.is_copy() { None } else { Some(self.state.owners.transfer(value)?) };
        if let Some(delta) = delta {
            self.state.facts.apply(delta);
        }
        self.state.counts[2] = self.state.counts[2].checked_add(1)?;
        self.push(Operation::VecPush { vector: source.place, value, cleanup }, ty, at, None);
        self.steps.last_mut()?.owners.extend(delta);
        Some(value)
    }

    fn available_push_target(&mut self, source: OwnedAggregatePlace, at: Span) -> Option<()> {
        let state = &self.state;
        let availability =
            AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
                state.parent(id)
            });
        if source.ty.category != TypeCategory::Vec
            || !source.mutable
            || !availability.projection_available(source.place, source.root)
            || state.moved.iter().any(|moved| availability.places_overlap(*moved, source.place))
        {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "push target is immutable, unavailable, or consumed during value preparation",
                "retain one complete mutable Vec until its prepared element is appended",
            );
            return None;
        }
        self.check_access(source.place, false, at)
    }
}
