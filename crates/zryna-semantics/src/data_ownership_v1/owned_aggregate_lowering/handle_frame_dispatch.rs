use super::super::handle_preparation::HandleReadSelection;
use super::{Frame, HandleOperation, PreparationContext, Span, Ty, VisitOutcome};

impl<'f> PreparationContext<'_, 'f, '_, '_> {
    pub(super) fn visit_handle_read(
        &mut self,
        id: u32,
        operand: Ty,
        result: Ty,
        operation: HandleOperation,
        at: Span,
        frames: &mut Vec<Frame<'f>>,
    ) -> Option<VisitOutcome> {
        match self.select_handle_read(id, operand, result, operation, at)? {
            HandleReadSelection::Value(value) => Some(VisitOutcome::Value(value)),
            HandleReadSelection::Deferred(frame) => {
                frames.push(Frame::HandleRead(frame));
                frames.push(Frame::Visit(id, Some(operand)));
                Some(VisitOutcome::Deferred)
            }
        }
    }
}
