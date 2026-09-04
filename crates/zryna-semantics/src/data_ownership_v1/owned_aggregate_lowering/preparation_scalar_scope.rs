use super::super::super::scalar_operations::{self, ScalarOperation};
use super::super::preparation_plan::{Operation, Step};
use super::{Frame, PreparationContext, Span, Ty, VisitOutcome};
use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;
use zryna_syntax::v4::RawIdentifierSyntax;

pub(super) struct ScalarFrame {
    pub(super) kind: ScalarOperation,
    pub(super) inputs: Vec<u32>,
    pub(super) values: Vec<(raw::ValueId, Ty)>,
    pub(super) expected: Option<Ty>,
    pub(super) ty: Ty,
    pub(super) at: Span,
    pub(super) start: usize,
    pub(super) next: usize,
    pub(super) waiting: bool,
}

impl<'f> PreparationContext<'_, 'f, '_, '_> {
    pub(super) fn inferred_clone(
        &mut self,
        id: u32,
        at: Span,
        frames: &mut Vec<Frame<'f>>,
    ) -> Option<VisitOutcome> {
        let expression = self.decisions.function.body.expressions.get(id as usize)?;
        let value = match &expression.kind {
            RawExpressionKind::Reference { name } => {
                let ty = self.inferred_reference_type(name)?;
                if ty.category == TypeCategory::String {
                    self.string_clone(id, ty, at)?
                } else {
                    self.aggregate_clone(id, ty, at)?
                }
            }
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                let source = self.resolve(id)?;
                if source.ty.category == TypeCategory::String {
                    self.resolved_string_clone(source, source.ty, at)?
                } else {
                    self.aggregate_clone(id, source.ty, at)?
                }
            }
            RawExpressionKind::StringLiteral { .. }
            | RawExpressionKind::Clone { .. }
            | RawExpressionKind::Call { .. } => {
                let ty = self.decisions.primitive(TypeCategory::String)?;
                frames.push(self.enter_string(super::StringOperation::Clone, vec![id], ty, at)?);
                return Some(VisitOutcome::Deferred);
            }
            _ => {
                super::super::clone_decisions::nonaddressable_clone(
                    super::super::super::span(self.decisions.input.sources(), expression.span),
                    self.decisions.errors,
                );
                return None;
            }
        };
        Some(VisitOutcome::Value(value))
    }

    pub(super) fn advance_scalar(
        &mut self,
        mut frame: ScalarFrame,
        result: &mut Option<raw::ValueId>,
        frames: &mut Vec<Frame<'f>>,
    ) -> Option<VisitOutcome> {
        if frame.waiting {
            let value = result.take()?;
            let step = self.steps.last()?;
            assert_eq!(step.value, Some(value), "scalar child owns its final typed result");
            frame.values.push((value, step.ty));
        }
        if let Some(&id) = frame.inputs.get(frame.next) {
            frame.next += 1;
            frame.waiting = true;
            frames.push(Frame::Scalar(frame));
            frames.push(Frame::Visit(id, None));
            Some(VisitOutcome::Deferred)
        } else {
            Some(VisitOutcome::Value(self.finish_scalar(frame)?))
        }
    }
    pub(super) fn inferred_reference_type(&mut self, name: &RawIdentifierSyntax) -> Option<Ty> {
        if let Some(binding) = self.bindings.get(&name.text) {
            return Some(binding.ty);
        }
        scalar_operations::missing_reference(
            &name.text,
            super::super::super::span(self.decisions.input.sources(), name.span),
            self.decisions.errors,
        );
        None
    }

    pub(super) fn enter_scalar(
        &mut self,
        kind: ScalarOperation,
        inputs: Vec<u32>,
        expected: Option<Ty>,
        ty: Ty,
        at: Span,
    ) -> Frame<'f> {
        assert!(self.state.summary, "scalar preparation belongs to mixed summary");
        let start = self.steps.len();
        self.push(
            Operation::ScalarEnter { kind, end: usize::MAX, operands: Vec::new() },
            ty,
            at,
            None,
        );
        Frame::Scalar(ScalarFrame {
            kind,
            inputs,
            values: Vec::new(),
            expected,
            ty,
            at,
            start,
            next: 0,
            waiting: false,
        })
    }

    pub(super) fn finish_scalar(&mut self, frame: ScalarFrame) -> Option<raw::ValueId> {
        let lhs = *frame.values.first()?;
        let rhs = frame.values.get(1).copied();
        frame.kind.validate(
            self.decisions.primitive(TypeCategory::I32),
            lhs.1,
            rhs.map(|(_, ty)| ty),
            frame.at,
            self.decisions.errors,
        )?;
        if let Some(expected) = frame.expected {
            scalar_operations::require_type(
                expected,
                frame.ty,
                frame.at,
                "scalar result",
                self.decisions.errors,
            )?;
        }
        let emission = self.state.emit(frame.ty, frame.at, self.decisions.errors)?;
        assert!(emission.owners.is_empty(), "scalar result has no owner effects");
        self.steps.push(Step {
            operation: Operation::ScalarCommit { kind: frame.kind, operands: frame.values.clone() },
            ty: frame.ty,
            at: frame.at,
            value: Some(emission.value),
            owners: emission.owners,
            after: self.state.checkpoint(),
        });
        let end = self.steps.len();
        let Operation::ScalarEnter { end: slot, operands, .. } =
            &mut self.steps[frame.start].operation
        else {
            unreachable!("scalar frame retains entry");
        };
        *slot = end;
        *operands = frame.values;
        Some(emission.value)
    }
}
