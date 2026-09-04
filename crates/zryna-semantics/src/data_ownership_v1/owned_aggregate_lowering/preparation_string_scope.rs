use super::super::super::owned_string_read::StringBytes;
use super::super::super::{Ty, span};
use super::super::preparation_operations::PreparationContext;
use super::super::preparation_plan::{Leaf, Operation, StringOperation, StringRead};
use super::{Frame, StringFrame};
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

pub(super) enum ReadSelection {
    Place(StringRead),
    Expression,
}

impl<'f> PreparationContext<'_, 'f, '_, '_> {
    pub(super) fn compound_string_read(&self, id: u32) -> Option<bool> {
        let expression = self.decisions.function.body.expressions.get(usize::try_from(id).ok()?)?;
        Some(!matches!(
            expression.kind,
            RawExpressionKind::Reference { .. }
                | RawExpressionKind::FieldAccess { .. }
                | RawExpressionKind::Index { .. }
        ))
    }

    pub(super) fn enter_string(
        &mut self,
        kind: StringOperation,
        inputs: Vec<u32>,
        ty: Ty,
        at: Span,
    ) -> Option<Frame<'f>> {
        self.require_string_scope(at)?;
        let start = self.steps.len();
        self.push(
            Operation::StringEnter { kind, end: usize::MAX, reads: Vec::new() },
            ty,
            at,
            None,
        );
        Some(Frame::String(StringFrame {
            kind,
            inputs,
            reads: Vec::new(),
            ty,
            at,
            start,
            next: 0,
            waiting: false,
        }))
    }

    pub(super) fn require_string_scope(&mut self, at: Span) -> Option<()> {
        if !self.state.summary {
            self.decisions.errors.at(
                "ZRYNA-M3016",
                at,
                "expression is outside private owned Struct/Enum/FixedArray lowering",
                "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
            );
            return None;
        }
        Some(())
    }

    pub(super) fn read_local_string(&mut self, id: u32, ty: Ty) -> Option<ReadSelection> {
        let expression = self.decisions.function.body.expressions.get(usize::try_from(id).ok()?)?;
        let at = span(self.decisions.input.sources(), expression.span);
        let (place, root, bytes) = match &expression.kind {
            RawExpressionKind::Reference { name } => {
                let (place, bytes) = super::super::super::owned_string_read::local_source(
                    name,
                    self.bindings,
                    &self.state.owners,
                    &self.state.facts.string_bytes,
                    Some(ty),
                    span(self.decisions.input.sources(), name.span),
                    self.decisions.errors,
                )?;
                (place, place, bytes)
            }
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                let source = self.string_read_projection(id, ty, at)?;
                (
                    source.place,
                    source.root,
                    StringBytes::from_known(
                        self.state.facts.string_bytes.get(&source.place).copied(),
                    ),
                )
            }
            _ => return Some(ReadSelection::Expression),
        };
        let read = StringRead { place, root, value: None, bytes };
        self.push(Operation::StringRead(read), ty, at, None);
        Some(ReadSelection::Place(read))
    }

    pub(super) fn read_result(
        &mut self,
        value: raw::ValueId,
        ty: Ty,
        at: Span,
    ) -> Option<StringRead> {
        let place = self.state.owners.owner(value)?;
        let bytes = StringBytes::from_known(self.state.facts.string_bytes.get(&place).copied());
        let read = StringRead { place, root: place, value: Some(value), bytes };
        self.push(Operation::StringRead(read), ty, at, None);
        Some(read)
    }

    pub(super) fn finish_string(&mut self, frame: StringFrame) -> Option<raw::ValueId> {
        let leaf = match (frame.kind, frame.reads.as_slice()) {
            (StringOperation::Clone, [source]) => {
                let source = super::super::super::type_model::OwnedAggregatePlace {
                    ty: frame.ty,
                    place: source.place,
                    root: source.root,
                    mutable: false,
                    is_root: source.place == source.root,
                };
                self.push(Operation::StringExit, frame.ty, frame.at, None);
                let cleanup = self.reverse(frame.ty, frame.at)?;
                Leaf::StringClone { source, bytes: frame.reads[0].bytes, cleanup }
            }
            (StringOperation::Concat, [left, right]) => {
                let bytes = super::super::super::owned_string_read::concat_optional_bytes(
                    left.bytes.known(),
                    right.bytes.known(),
                    frame.at,
                    self.decisions.errors,
                )?;
                self.push(Operation::StringExit, frame.ty, frame.at, None);
                let cleanup = self.reverse(frame.ty, frame.at)?;
                Leaf::StringConcat { left: left.place, right: right.place, bytes, cleanup }
            }
            _ => return None,
        };
        let value = self.emit_leaf(leaf, frame.ty, frame.at)?;
        self.steps[frame.start].operation =
            Operation::StringEnter { kind: frame.kind, end: self.steps.len(), reads: frame.reads };
        Some(value)
    }
}
