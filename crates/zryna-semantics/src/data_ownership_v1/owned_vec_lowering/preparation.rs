use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::owned_lowering_resources::{
    OwnedStringPreparationBudget, preflight_owned_string_preparation,
};
use super::super::span;
use super::super::string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringEstimateError, OwnedStringPreparationEstimate,
    VecPreparationEstimate, add_estimate_counts, cleanup_actions_after_additions,
    cleanup_actions_after_preparation, empty_owned_string_estimate,
    estimate_owned_string_expression,
};
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn expression(
        &self,
        id: u32,
    ) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    pub(in crate::data_ownership_v1) fn preflight_string_expression(
        &mut self,
        id: u32,
        string_ty: Ty,
        at: Span,
    ) -> bool {
        let estimate = match estimate_owned_string_expression(
            self.function,
            &self.bindings,
            &self.owners,
            string_ty,
            id,
            self.owners.pending().len(),
            OwnedStringEstimateContext::Value,
        ) {
            Ok(estimate) => estimate,
            Err(OwnedStringEstimateError::Unsupported) => return true,
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3014",
                    span(self.input.sources(), reference),
                    "Vec String element has no available owner",
                    "move each String element at most once",
                );
                return false;
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive Vec String-element preparation overflows its checked resource estimate",
                    "reduce nested String-producing element expressions",
                );
                return false;
            }
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn estimate_string_sequence(
        &mut self,
        expressions: &[u32],
        string_ty: Ty,
        at: Span,
    ) -> Option<OwnedStringPreparationEstimate> {
        let mut total = empty_owned_string_estimate(self.owners.pending().len());
        for expression in expressions {
            let child = match estimate_owned_string_expression(
                self.function,
                &self.bindings,
                &self.owners,
                string_ty,
                *expression,
                total.end_pending,
                OwnedStringEstimateContext::Value,
            ) {
                Ok(estimate) => estimate,
                Err(OwnedStringEstimateError::Unsupported) => {
                    let expression = self.expression(*expression)?;
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), expression.span),
                        "Vec<String> element expression is outside checked String preparation",
                        "use a String literal, available String move, clone, concat, or private String call",
                    );
                    return None;
                }
                Err(OwnedStringEstimateError::Unavailable(reference)) => {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), reference),
                        "Vec String element has no available owner",
                        "move each String element at most once",
                    );
                    return None;
                }
                Err(OwnedStringEstimateError::Overflow) => {
                    self.errors.at(
                        "ZRYNA-M3201",
                        at,
                        "recursive Vec String-element preparation overflows its checked resource estimate",
                        "reduce nested String-producing element expressions",
                    );
                    return None;
                }
            };
            total = match add_estimate_counts(total, child) {
                Ok(total) => total,
                Err(OwnedStringEstimateError::Overflow) => {
                    self.errors.at(
                        "ZRYNA-M3201",
                        at,
                        "recursive Vec String-element sequence overflows its checked resource estimate",
                        "reduce nested String-producing element expressions",
                    );
                    return None;
                }
                Err(
                    OwnedStringEstimateError::Unsupported
                    | OwnedStringEstimateError::Unavailable(_),
                ) => {
                    unreachable!("combining checked estimates cannot change expression support")
                }
            };
        }
        Some(total)
    }

    pub(in crate::data_ownership_v1) fn preflight_string_sequence_with_enclosing_cleanup(
        &mut self,
        estimate: OwnedStringPreparationEstimate,
        enclosing_actions: usize,
        at: Span,
    ) -> bool {
        let Some(reserved_cleanup_plans) = self.reserved_cleanup_plans.checked_add(1) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "enclosing Vec cleanup reservation overflows its checked resource estimate",
                "reduce nested Vec and String-producing expressions",
            );
            return false;
        };
        let Some(reserved_cleanup_actions) =
            self.reserved_cleanup_actions.checked_add(enclosing_actions)
        else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "enclosing Vec cleanup action reservation overflows its checked resource estimate",
                "reduce simultaneously live Vec and String owners",
            );
            return false;
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn estimate_vec_preparation(
        &mut self,
        id: u32,
        expected: Ty,
        pending: usize,
        at: Span,
    ) -> Option<VecPreparationEstimate> {
        let expression = self.expression(id)?.clone();
        match expression.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).filter(|binding| {
                    binding.ty == expected && self.owners.contains(binding.place)
                })?;
                Some(VecPreparationEstimate {
                    end_pending: pending,
                    resources: OwnedStringPreparationEstimate {
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: true,
                        ..empty_owned_string_estimate(pending)
                    },
                })
            }
            RawExpressionKind::Clone { value, .. } => {
                let operand = self.expression(value)?;
                let RawExpressionKind::Reference { name } = &operand.kind else {
                    return None;
                };
                self.bindings.get(&name.text).filter(|binding| {
                    binding.ty == expected && self.owners.contains(binding.place)
                })?;
                let end_pending = pending.checked_add(1)?;
                let clones_non_copy_elements = self.element.category == TypeCategory::String;
                Some(VecPreparationEstimate {
                    end_pending,
                    resources: OwnedStringPreparationEstimate {
                        end_pending,
                        peak_pending: end_pending,
                        cleanup_plans: 1 + usize::from(clones_non_copy_elements),
                        cleanup_actions: pending.checked_add(if clones_non_copy_elements {
                            pending.checked_add(1)?
                        } else {
                            0
                        })?,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: false,
                        root_cleanup_actions: Some(pending),
                    },
                })
            }
            RawExpressionKind::VecConstruction { elements, .. } => {
                let mut resources = if self.element.category == TypeCategory::String {
                    self.estimate_string_sequence(&elements, self.element, at)?
                } else {
                    empty_owned_string_estimate(pending)
                };
                let prepared_pending = resources.end_pending;
                let consumed = usize::from(!self.element.is_copy()) * elements.len();
                let end_pending = prepared_pending.checked_sub(consumed)?.checked_add(1)?;
                resources.end_pending = end_pending;
                resources.peak_pending = resources.peak_pending.max(end_pending);
                resources.cleanup_plans = resources.cleanup_plans.checked_add(1)?;
                resources.cleanup_actions =
                    resources.cleanup_actions.checked_add(prepared_pending)?;
                resources.values = resources.values.checked_add(1)?;
                resources.places = resources.places.checked_add(1)?;
                resources.transitions = resources.transitions.checked_add(1)?;
                resources.root_cleanup_actions = Some(prepared_pending);
                Some(VecPreparationEstimate { end_pending, resources })
            }
            RawExpressionKind::Call { arguments, .. } if arguments.is_empty() => {
                let end_pending = pending.checked_add(1)?;
                Some(VecPreparationEstimate {
                    end_pending,
                    resources: OwnedStringPreparationEstimate {
                        end_pending,
                        peak_pending: end_pending,
                        cleanup_plans: 1,
                        cleanup_actions: pending,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        transfers_existing: false,
                        root_cleanup_actions: Some(pending),
                    },
                })
            }
            RawExpressionKind::Call { arguments, .. } if arguments.len() == 1 => {
                let mut preparation =
                    self.estimate_vec_preparation(arguments[0], expected, pending, at)?;
                let cleanup = preparation.end_pending.checked_sub(1)?;
                preparation.resources.cleanup_plans =
                    preparation.resources.cleanup_plans.checked_add(1)?;
                preparation.resources.cleanup_actions =
                    preparation.resources.cleanup_actions.checked_add(cleanup)?;
                preparation.resources.values = preparation.resources.values.checked_add(1)?;
                preparation.resources.places = preparation.resources.places.checked_add(1)?;
                preparation.resources.transitions =
                    preparation.resources.transitions.checked_add(1)?;
                preparation.resources.root_cleanup_actions = Some(cleanup);
                Some(preparation)
            }
            _ => None,
        }
    }

    pub(in crate::data_ownership_v1) fn preflight_push_cleanup(
        &mut self,
        value: u32,
        at: Span,
    ) -> Option<usize> {
        let moves_existing_owner = !self.element.is_copy()
            && self.expression(value).is_some_and(|expression| {
                matches!(&expression.kind, RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| {
                    binding.ty == self.element && self.owners.contains(binding.place)
                }))
            });
        let nested_estimate = if self.element.category == TypeCategory::String {
            Some(self.estimate_string_sequence(&[value], self.element, at)?)
        } else {
            None
        };
        let reserved_actions = nested_estimate.map_or_else(
            || {
                cleanup_actions_after_preparation(
                    self.owners.pending().len(),
                    !self.element.is_copy() && !moves_existing_owner,
                )
            },
            |estimate| estimate.end_pending,
        );
        if let Some(estimate) = nested_estimate
            && !self.preflight_string_sequence_with_enclosing_cleanup(
                estimate,
                reserved_actions,
                at,
            )
        {
            return None;
        }
        Some(reserved_actions)
    }

    pub(in crate::data_ownership_v1) fn preflight_construct_cleanup(
        &mut self,
        elements: &[u32],
        at: Span,
    ) -> Option<usize> {
        let nested_estimate = if self.element.category == TypeCategory::String {
            Some(self.estimate_string_sequence(elements, self.element, at)?)
        } else {
            None
        };
        let additional_owners = elements
            .iter()
            .filter(|element| {
                !self.element.is_copy()
                    && !self.expression(**element).is_some_and(|expression| {
                        matches!(&expression.kind, RawExpressionKind::Reference { name }
                        if self.bindings.get(&name.text).is_some_and(|binding| {
                            binding.ty == self.element && self.owners.contains(binding.place)
                        }))
                    })
            })
            .count();
        let reserved_actions = nested_estimate.map_or_else(
            || cleanup_actions_after_additions(self.owners.pending().len(), additional_owners),
            |estimate| estimate.end_pending,
        );
        if let Some(estimate) = nested_estimate
            && !self.preflight_string_sequence_with_enclosing_cleanup(
                estimate,
                reserved_actions,
                at,
            )
        {
            return None;
        }
        Some(reserved_actions)
    }
}
