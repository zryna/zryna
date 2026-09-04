use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_constructor_plan::ConstructorKind;
use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;
use super::expression_decisions::{
    ArrayDecision, EnumDecision, ExpressionDecisions, ExpressionKind, StructDecision,
};

impl<'a, 'f> PrivateOwnedAggregateLowerer<'a, 'f, '_> {
    fn expression_decisions(&mut self) -> ExpressionDecisions<'a, 'f, '_> {
        ExpressionDecisions {
            input: self.input,
            file: self.file,
            function: self.function,
            module: self.module,
            declarations: self.declarations,
            graph: self.graph,
            node_types: self.node_types,
            layouts: self.layouts,
            errors: self.errors,
        }
    }

    pub(super) fn value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let decision = self.expression_decisions().classify(id, expected)?;
        let at = decision.at;
        match decision.kind {
            ExpressionKind::Bool(value) => {
                self.emit(expected, at, raw::InstructionKind::BoolLiteral(value))
            }
            ExpressionKind::I32(value) => {
                self.emit(expected, at, raw::InstructionKind::I32Literal(value))
            }
            ExpressionKind::String(bytes) => {
                let bytes = bytes.to_vec();
                let cleanup = self.push_cleanup(at, None)?;
                self.emit(expected, at, raw::InstructionKind::StringFromUtf8 { bytes, cleanup })
            }
            ExpressionKind::Reference(name) => self.reference_value(name, expected, at),
            ExpressionKind::Projection(id) => self.projected_value(id, expected, None),
            ExpressionKind::StringClone(value) => self.clone_projected_string(value, expected, at),
            ExpressionKind::AggregateClone(value) => self.clone_aggregate(value, expected, at),
            ExpressionKind::Struct(decision) => self.struct_value(decision, expected, at),
            ExpressionKind::Array(decision) => self.array_value(&decision, expected, at),
            ExpressionKind::Enum(decision) => self.prepare_enum_payload(expected, &decision, at),
        }
    }

    fn struct_value(
        &mut self,
        decision: StructDecision,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let reservation = self.reserve_constructor_commit(expected, decision.children.len(), at)?;
        let values = (|| {
            let mut values = Vec::with_capacity(decision.children.len());
            for (type_syntax, expression) in decision.children {
                let field_ty = self.expression_decisions().child_type(type_syntax)?;
                values.push(self.value(expression, field_ty)?);
            }
            Some(values)
        })();
        reservation.release(self);
        self.commit_constructor(expected, ConstructorKind::Struct, &values?, at)
    }

    fn array_value(
        &mut self,
        decision: &ArrayDecision<'f>,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let reservation = self.reserve_constructor_commit(expected, decision.elements.len(), at)?;
        let values = (|| {
            let mut values = Vec::with_capacity(decision.elements.len());
            for expression in decision.elements {
                values.push(self.value(*expression, decision.element)?);
            }
            Some(values)
        })();
        reservation.release(self);
        self.commit_constructor(expected, ConstructorKind::FixedArray, &values?, at)
    }

    fn prepare_enum_payload(
        &mut self,
        expected: Ty,
        decision: &EnumDecision,
        at: Span,
    ) -> Option<raw::ValueId> {
        let reservation = self.reserve_constructor_commit(
            expected,
            usize::from(decision.payload_input.is_some()),
            at,
        )?;
        let payload_value = match decision.payload_input {
            Some((expression, ty)) => self.value(expression, ty).map(Some),
            None => Some(None),
        };
        reservation.release(self);
        self.commit_enum(expected, at, decision.ordinal, payload_value?)
    }
}
