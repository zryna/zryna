use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;

use super::Ty;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Leaf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HandleOperation {
    SharedClone,
    WeakDowngrade,
    WeakClone,
}

impl PreparationContext<'_, '_, '_, '_> {
    fn available_handle(
        &mut self,
        source: super::super::type_model::OwnedAggregatePlace,
        at: Span,
    ) -> Option<()> {
        let state = &self.state;
        let availability = super::availability::AvailabilityView::new(
            &state.owners,
            &state.moved,
            &state.partial,
            |id| state.parent(id),
        );
        if !availability.projection_available(source.place, source.root)
            || state.moved.iter().any(|moved| availability.places_overlap(*moved, source.place))
        {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "shared or weak handle is moved or unavailable",
                "use one complete initialized handle before moving it",
            );
            return None;
        }
        self.check_access(source.place, true, at)
    }

    pub(super) fn handle_payload(&self, handle: Ty) -> Option<Ty> {
        let payload = self.decisions.layouts.type_by_id(handle.layout)?.referenced_type()?;
        self.decisions.node_types.iter().flatten().find(|ty| ty.layout == payload).copied()
    }

    pub(super) fn handle_read(
        &mut self,
        id: u32,
        operand: Ty,
        result: Ty,
        operation: HandleOperation,
        at: Span,
    ) -> Option<raw::ValueId> {
        let source = self.resolve(id)?;
        let valid = source.ty == operand
            && match operation {
                HandleOperation::SharedClone => {
                    operand.category == TypeCategory::Shared && result == operand
                }
                HandleOperation::WeakClone => {
                    operand.category == TypeCategory::Weak && result == operand
                }
                HandleOperation::WeakDowngrade => {
                    operand.category == TypeCategory::Shared
                        && result.category == TypeCategory::Weak
                        && self.handle_payload(operand) == self.handle_payload(result)
                }
            };
        if !valid {
            self.decisions.errors.at(
                "ZRYNA-M3013",
                at,
                "shared or weak operation has the wrong exact handle type",
                "clone one exact handle or downgrade Shared<T> to Weak<T>",
            );
            return None;
        }
        self.available_handle(source, at)?;
        let cleanup = self.reverse(result, at)?;
        let leaf = match operation {
            HandleOperation::SharedClone => Leaf::SharedClone { source: source.place, cleanup },
            HandleOperation::WeakDowngrade => Leaf::WeakDowngrade { source: source.place, cleanup },
            HandleOperation::WeakClone => Leaf::WeakClone { source: source.place, cleanup },
        };
        self.emit_leaf(leaf, result, at)
    }

    pub(super) fn shared_construct(
        &mut self,
        value: raw::ValueId,
        result: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        if result.category != TypeCategory::Shared {
            return None;
        }
        let cleanup = self.reverse(result, at)?;
        self.emit_leaf(Leaf::SharedConstruct { value, cleanup }, result, at)
    }
}
