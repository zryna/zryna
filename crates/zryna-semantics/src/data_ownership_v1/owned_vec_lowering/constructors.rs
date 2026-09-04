use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::layout_graph::semantic_type;
use super::super::owned_constructor_plan::{ConstructorKind, ConstructorShape};
use super::super::owner_state::apply_owner_delta;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

struct VecConstructorReservation {
    cleanup_actions: usize,
}

impl VecConstructorReservation {
    fn release(self, lowerer: &mut PrivateVecLowerer<'_, '_, '_>) {
        lowerer.release_cleanup_capacity(self.cleanup_actions);
        lowerer.cfg.release_transitions(1);
        lowerer.release_local_place();
        lowerer.cfg.release_values(1);
    }
}

impl PrivateVecLowerer<'_, '_, '_> {
    fn reserve_constructor(
        &mut self,
        actions: usize,
        at: Span,
    ) -> Option<VecConstructorReservation> {
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if !self.reserve_cleanup_capacity(actions, at) {
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        Some(VecConstructorReservation { cleanup_actions: actions })
    }

    pub(super) fn construct_vec(
        &mut self,
        type_syntax: u32,
        elements: &[u32],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let constructed = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if constructed != expected {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "Vec construction type differs from its contextual type",
                "construct the exact annotated Vec type",
            );
            return None;
        }
        let actions = self.preflight_construct_cleanup(elements, at)?;
        let ticket = self.reserve_constructor(actions, at)?;
        let prepared = (|| {
            let mut values = Vec::with_capacity(elements.len());
            for element in elements {
                values.push(self.value(*element, self.element)?);
            }
            let prepared = ConstructorShape::derive(
                self.layouts,
                expected,
                ConstructorKind::Vec,
                values.len(),
                |id| self.node_types.iter().flatten().find(|ty| ty.layout == id).copied(),
            )
            .and_then(|shape| {
                shape.prepare(
                    &values,
                    |value| self.cfg.value_types.get(value.0 as usize).copied(),
                    &self.owners,
                )
            });
            if let Ok(prepared) = prepared {
                Some(prepared)
            } else {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "Vec constructor operand owner is unavailable before commit",
                    "construct from currently pending exact values, moving each owned element once",
                );
                None
            }
        })();
        ticket.release(self);
        let prepared = prepared?;
        let cleanup = self.push_instruction_cleanup(at, None)?;
        let instruction = prepared.instruction(Some(cleanup)).expect("Vec constructor cleanup");
        let result = self.emit(prepared.result_type(), at, instruction)?.0;
        for delta in prepared.commit(&mut self.owners) {
            apply_owner_delta(&mut self.known_string_bytes, delta);
        }
        Some(result)
    }
}
