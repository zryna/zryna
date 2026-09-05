use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::function_catalog::FunctionResolution;
use super::super::type_model::OwnedAggregatePlace;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;
use super::preparation_state::PlannedPlace;

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn fresh_indexed_type(&mut self, id: u32) -> Option<Ty> {
        match &self.decisions.function.body.expressions.get(id as usize)?.kind {
            RawExpressionKind::Call { callee, .. } => {
                let FunctionResolution::Exact(signature) =
                    self.catalog.resolve(self.decisions.module, &callee.text)
                else {
                    return None;
                };
                Some(signature.result)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, .. } => {
                self.decisions.child_type(*type_syntax)
            }
            _ => None,
        }
    }

    pub(super) fn materialize_indexed_base(
        &mut self,
        id: u32,
        ty: Ty,
        at: Span,
    ) -> Option<OwnedAggregatePlace> {
        let value = self.walk(id, ty)?;
        let place = if ty.is_copy() {
            let place = raw::PlaceId(u32::try_from(self.state.counts[1]).ok()?);
            self.state.counts[1] = self.state.counts[1].checked_add(1)?;
            self.state.places.push(PlannedPlace { ty, at, kind: raw::PlaceKind::Temporary(value) });
            self.state.effect()?;
            self.push(Operation::IndexedCopyStorage { place, value }, ty, at, None);
            place
        } else {
            self.state.owners.owner(value)?
        };
        Some(OwnedAggregatePlace { ty, place, root: place, mutable: false, is_root: true })
    }

    pub(super) fn drop_indexed_base(
        &mut self,
        source: OwnedAggregatePlace,
        at: Span,
    ) -> Option<()> {
        if !source.ty.is_copy() {
            let delta = self.state.owners.consume_owner(source.place)?;
            self.state.facts.apply(delta);
            self.indexed_effect(
                raw::InstructionKind::DropPlace { place: source.place },
                source.ty,
                at,
            )?;
            self.steps.last_mut()?.owners.push(delta);
        }
        Some(())
    }
}
