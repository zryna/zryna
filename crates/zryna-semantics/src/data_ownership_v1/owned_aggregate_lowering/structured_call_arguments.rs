use std::collections::BTreeSet;
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::function_catalog::FunctionSignature;
use super::preparation_plan::{CallParameter, call_parameters};
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, Ty};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_call_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[u32],
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<Vec<raw::CallArgument>> {
        let mut ordered = Vec::with_capacity(arguments.len());
        let mut owned = Vec::new();
        let mut exclusive = BTreeSet::new();
        for (&id, parameter) in arguments.iter().zip(call_parameters(signature)) {
            match parameter {
                CallParameter::Value(ty) => {
                    let value = self.structured_value(id, ty, graph)?;
                    ordered.push(raw::CallArgument::Value(value));
                    owned.push((value, ty));
                }
                CallParameter::Borrow { ty, access } => {
                    let borrow = self.structured_borrow_argument(id, ty, access)?;
                    if access == raw::BorrowAccess::Exclusive && !exclusive.insert(borrow) {
                        self.errors.at(
                            "ZRYNA-M3017",
                            at,
                            "call repeats one exclusive borrow authority",
                            "pass each exclusive authority at most once per call",
                        );
                        return None;
                    }
                    ordered.push(raw::CallArgument::Borrow(borrow));
                }
            }
        }
        let types = self.constructor_types.observed_snapshot(&self.instructions).ok()?;
        for (value, ty) in owned {
            if types.get(value) != Some(ty.ir) {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "structured call operand has the wrong exact type",
                    "pass the exact declared argument types",
                );
                return None;
            }
            if !ty.is_copy() {
                let Some(delta) = self.owners.transfer(value) else {
                    self.errors.at("ZRYNA-M3014", at, "structured call operand owner is unavailable", "retain each distinct initialized argument until the complete call is prepared");
                    return None;
                };
                self.preparation_facts.apply(delta);
            }
        }
        let (mut values, borrows): (Vec<_>, Vec<_>) = ordered
            .into_iter()
            .partition(|argument| matches!(argument, raw::CallArgument::Value(_)));
        values.extend(borrows);
        Some(values)
    }

    fn structured_borrow_argument(
        &mut self,
        id: u32,
        ty: Ty,
        access: raw::BorrowAccess,
    ) -> Option<raw::BorrowId> {
        let expression = self.expression(id)?;
        let at = super::super::diagnostics::span(self.input.sources(), expression.span);
        if let RawExpressionKind::Reference { name } = &expression.kind
            && let Some(alias) = self.preparation_facts.aliases.get(&name.text)
            && alias.ty == ty
            && alias.access == access
            && self.preparation_facts.borrow_active(alias.borrow)
        {
            return Some(alias.borrow);
        }
        self.errors.at(
            "ZRYNA-M3017",
            at,
            "structured call requires an exact live call-frame borrow parameter",
            "forward one matching formal authority without ending or reborrowing it across a match",
        );
        None
    }
}
