use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_constructor_plan::ConstructorKind;
use super::constructor_resources::ConstructorCommitReservation;
use super::expression_decisions::{ExpressionDecisions, ExpressionKind};
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, Ty};

struct ConstructorFrame {
    kind: ConstructorKind,
    ty: Ty,
    at: Span,
    children: Vec<(u32, Ty)>,
    values: Vec<raw::ValueId>,
    reservation: ConstructorCommitReservation,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_operand(
        &mut self,
        id: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        use zryna_syntax::v4::RawExpressionKind;
        let expression = self.expression(id)?;
        if graph.contains_match(expression.span.start, expression.span.end) {
            match expression.kind {
                RawExpressionKind::Call { .. } => return self.structured_call(id, ty, graph),
                RawExpressionKind::Clone { .. }
                    if ty.category == zryna_layout::TypeCategory::String =>
                {
                    return self.structured_string(id, ty, graph);
                }
                RawExpressionKind::StructConstruction { .. }
                | RawExpressionKind::EnumConstruction { .. }
                | RawExpressionKind::FixedArrayConstruction { .. }
                | RawExpressionKind::VecConstruction { .. } => {
                    return self.structured_constructor(id, ty, graph);
                }
                _ => {}
            }
        }
        self.value(id, ty)
    }

    fn structured_constructor_frame(&mut self, id: u32, ty: Ty) -> Option<ConstructorFrame> {
        let mut decisions = ExpressionDecisions {
            input: self.input,
            file: self.file,
            function: self.function,
            module: self.module,
            declarations: self.declarations,
            graph: self.graph,
            node_types: self.node_types,
            layouts: self.layouts,
            errors: self.errors,
        };
        let decision = decisions.classify_prepared(id, Some(ty), true)?;
        let (kind, children) = match decision.kind {
            ExpressionKind::Struct(value) => {
                let children = value
                    .children
                    .into_iter()
                    .map(|(syntax, child)| Some((child, decisions.child_type(syntax)?)))
                    .collect::<Option<Vec<_>>>()?;
                (ConstructorKind::Struct, children)
            }
            ExpressionKind::Array(value) => (
                ConstructorKind::FixedArray,
                value.elements.iter().map(|&child| (child, value.element)).collect(),
            ),
            ExpressionKind::Enum(value) => (
                ConstructorKind::Enum { variant: u32::try_from(value.ordinal).ok()? },
                value.payload_input.into_iter().collect(),
            ),
            ExpressionKind::Vec(value) => (
                ConstructorKind::Vec,
                value.elements.iter().map(|&child| (child, value.element)).collect(),
            ),
            _ => return None,
        };
        let reservation = self.reserve_constructor_commit(ty, children.len(), decision.at)?;
        Some(ConstructorFrame {
            kind,
            ty,
            at: decision.at,
            values: Vec::with_capacity(children.len()),
            children,
            reservation,
        })
    }

    pub(super) fn structured_constructor(
        &mut self,
        id: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        let mut frame = self.structured_constructor_frame(id, ty)?;
        for &(child, child_ty) in &frame.children {
            frame.values.push(self.structured_value(child, child_ty, graph)?);
        }
        frame.reservation.release(self);
        let cleanup = if frame.kind == ConstructorKind::Vec {
            Some(self.push_cleanup(frame.at, None)?)
        } else {
            None
        };
        Some(
            self.commit_constructor_with_cleanup(
                frame.ty,
                frame.kind,
                &frame.values,
                frame.at,
                cleanup,
            )?
            .value,
        )
    }
}
