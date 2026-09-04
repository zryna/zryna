use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::super::super::owned_call_resolution::OwnedCallResolution;
use super::super::super::owner_state::OwnerDelta;
use super::super::preparation_plan::{CallKind, CallSignature};
use super::{Frame, Operation, PreparationContext, Ty};

pub(super) struct CallFrame {
    pub(super) signature: CallSignature,
    pub(super) inputs: Vec<u32>,
    pub(super) values: Vec<raw::ValueId>,
    pub(super) at: Span,
    pub(super) start: usize,
    pub(super) next: usize,
    pub(super) waiting: bool,
    reservation: super::ConstructorCommitReservation,
}

impl<'f> PreparationContext<'_, 'f, '_, '_> {
    fn call_kind(&mut self, ty: Ty, at: Span) -> Option<CallKind> {
        Some(match ty.category {
            TypeCategory::String => CallKind::String,
            TypeCategory::Vec
                if self
                    .decisions
                    .layouts
                    .type_by_id(ty.layout)
                    .and_then(zryna_layout::VerifiedType::referenced_type)
                    .and_then(|value| self.decisions.layouts.type_by_id(value))
                    .is_some_and(|value| value.category() == TypeCategory::String) =>
            {
                CallKind::Vec
            }
            _ => {
                self.decisions.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "mixed constructor calls require an existing String or Vec<String> signature",
                    "call an exact private String or Vec<String> producer or identity function",
                );
                return None;
            }
        })
    }

    pub(super) fn enter_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        expected: Option<Ty>,
        at: Span,
    ) -> Option<Frame<'f>> {
        if !self.state.summary {
            self.decisions.errors.at(
                "ZRYNA-M3016",
                at,
                "expression is outside private owned Struct/Enum/FixedArray lowering",
                "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
            );
            return None;
        }
        let inferred = if expected.is_none() {
            Some(
                OwnedCallResolution {
                    input: self.decisions.input,
                    module: self.decisions.module,
                    catalog: self.catalog,
                    errors: self.decisions.errors,
                }
                .lookup(callee, "call one exact private same-module function")?,
            )
        } else {
            None
        };
        let ty = expected.or_else(|| inferred.as_ref().map(|signature| signature.result))?;
        let kind = self.call_kind(ty, at)?;
        let mut resolver = OwnedCallResolution {
            input: self.decisions.input,
            module: self.decisions.module,
            catalog: self.catalog,
            errors: self.decisions.errors,
        };
        let signature = match (kind, inferred) {
            (CallKind::String, Some(signature)) => resolver.checked_string(ty, callee, signature),
            (CallKind::Vec, Some(signature)) => resolver.checked_vec(ty, callee, ty, signature),
            (CallKind::String, None) => resolver.string(ty, callee),
            (CallKind::Vec, None) => resolver.vec(ty, callee, ty),
        }?;
        if arguments.len() != signature.parameters.len() {
            self.decisions.errors.at(
                if kind == CallKind::String { "ZRYNA-M3012" } else { "ZRYNA-M3016" },
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                if kind == CallKind::String {
                    "pass the exact declared String argument"
                } else {
                    "pass the exact declared Vec argument"
                },
            );
            return None;
        }
        let signature = CallSignature {
            id: signature.id,
            result: signature.result,
            parameter: signature.parameters.first().copied(),
            kind,
            bytes: (kind == CallKind::String)
                .then_some(super::super::super::owned_string_read::StringBytes::Unknown),
        };
        let reservation = self.state.ledger().acquire_constructor(0, 1)?;
        let start = self.steps.len();
        self.push(
            Operation::CallEnter { signature, end: usize::MAX, arguments: Vec::new() },
            ty,
            at,
            None,
        );
        Some(Frame::Call(CallFrame {
            signature,
            inputs: arguments.to_vec(),
            values: Vec::new(),
            at,
            start,
            next: 0,
            waiting: false,
            reservation,
        }))
    }

    pub(super) fn finish_call(&mut self, frame: CallFrame) -> Option<raw::ValueId> {
        let ty = frame.signature.result;
        for &value in &frame.values {
            let owner = self.state.owners.owner(value)?;
            let delta = self.state.owners.transfer(value)?;
            assert_eq!(delta, OwnerDelta::Transferred { owner }, "prepared call transfer owner");
            self.state.facts.apply(delta);
            self.push(Operation::CallTransfer { value, owner }, ty, frame.at, None);
            self.steps.last_mut()?.owners.push(delta);
        }
        self.state.ledger().release_constructor(frame.reservation);
        self.push(Operation::CallRelease, ty, frame.at, None);
        let cleanup = self.reverse(ty, frame.at)?;
        let emission = self.state.emit(ty, frame.at, self.decisions.errors)?;
        for delta in &emission.owners {
            self.state.facts.apply(*delta);
        }
        self.push(
            Operation::CallCommit {
                signature: frame.signature,
                arguments: frame.values.clone(),
                cleanup,
            },
            ty,
            frame.at,
            Some(emission.value),
        );
        self.steps.last_mut()?.owners = emission.owners;
        let end = self.steps.len();
        let Operation::CallEnter { end: slot, arguments, .. } =
            &mut self.steps[frame.start].operation
        else {
            unreachable!("call frame retains entry");
        };
        *slot = end;
        *arguments = frame.values;
        Some(emission.value)
    }
}
