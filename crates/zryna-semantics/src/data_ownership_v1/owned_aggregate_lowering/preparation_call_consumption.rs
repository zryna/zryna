use super::super::super::super::{Ty, owner_state::OwnerDelta};
use super::super::super::constructor_resources::ConstructorCommitReservation;
use super::super::super::preparation_plan::{CallKind, CallSignature};
use super::Consumption;
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

struct OpenCall {
    start: usize,
    end: usize,
    depth: usize,
    signature: CallSignature,
    arguments: Vec<raw::ValueId>,
    actual: Vec<raw::ValueId>,
    transfers: usize,
    reservation: Option<ConstructorCommitReservation>,
    actions: usize,
}

#[derive(Default)]
pub(super) struct CallScopes {
    open: Vec<OpenCall>,
    released: Option<OpenCall>,
}

impl CallScopes {
    pub(super) fn start(&self) -> Option<usize> {
        self.open.last().map(|call| call.start)
    }
    pub(super) fn result(&mut self, value: raw::ValueId, depth: usize) -> bool {
        let call = self.open.last_mut().expect("call output owns a scope");
        if call.depth != depth {
            return true;
        }
        assert!(call.actual.len() < call.arguments.len(), "call has no extra argument result");
        call.actual.push(value);
        false
    }
    pub(super) fn pending(&self) -> bool {
        self.released.is_some()
    }
    pub(super) fn complete(&self) -> bool {
        self.open.is_empty() && self.released.is_none()
    }
}

impl Consumption<'_, '_, '_, '_> {
    pub(super) fn enter_call(
        &mut self,
        range: (usize, usize, usize),
        signature: CallSignature,
        arguments: Vec<raw::ValueId>,
        (ty, at): (Ty, Span),
        actions: usize,
    ) {
        let (start, end, length) = range;
        assert!(self.cleanups.is_empty(), "call cannot interrupt cleanup");
        assert!(self.calls.released.is_none(), "call release must finish before another scope");
        assert!(end > start + 3 && end <= length, "call exact range");
        assert_eq!(
            signature.id.module.0 as usize, self.lowerer.module,
            "call same module authority"
        );
        let actual = self
            .lowerer
            .catalog
            .modules
            .get(signature.id.module.0 as usize)
            .and_then(|module| module.get(signature.id.declaration as usize))
            .and_then(Option::as_ref)
            .expect("call catalog identity");
        assert_eq!(actual.id, signature.id, "call actual callee identity");
        assert!(actual.private && !actual.has_borrow_parameters(), "call private value signature");
        assert_eq!(actual.result, ty, "call actual result type");
        assert_eq!(signature.result, ty, "call recorded result type");
        assert_eq!(
            actual.parameters.as_slice(),
            signature.parameter.as_slice(),
            "call actual parameter signature"
        );
        assert_eq!(
            arguments.len(),
            usize::from(signature.parameter.is_some()),
            "call exact argument arity"
        );
        assert!(
            signature.parameter.is_none_or(|parameter| parameter == ty),
            "call exact identity parameter"
        );
        assert_eq!(
            signature.kind == CallKind::String,
            ty.category == zryna_layout::TypeCategory::String,
            "call category linkage"
        );
        if signature.kind == CallKind::Vec {
            assert!(ty.category == zryna_layout::TypeCategory::Vec
                && self.lowerer.layouts.type_by_id(ty.layout)
                    .and_then(zryna_layout::VerifiedType::referenced_type)
                    .and_then(|value| self.lowerer.layouts.type_by_id(value))
                    .is_some_and(|value| value.category() == zryna_layout::TypeCategory::String),
                "call exact Vec String authority");
        }
        assert_eq!(
            signature.bytes,
            (signature.kind == CallKind::String)
                .then_some(super::super::super::super::owned_string_read::StringBytes::Unknown),
            "call opaque result byte witness"
        );
        if let Some(parent) = self.calls.open.last() {
            assert!(end < parent.end, "nested call range");
        }
        let reservation = self
            .lowerer
            .reserve_constructor_commit(ty, 0, at)
            .expect("prepared call result reservation");
        self.lowerer.preparation_facts.held_cleanup = super::super::call_resources::reserve(
            self.lowerer.preparation_checkpoint(),
            actions,
            signature.kind,
            at,
            self.lowerer.errors,
        )
        .expect("prepared call cleanup reservation");
        self.calls.open.push(OpenCall {
            start,
            end,
            depth: self.open.len(),
            signature,
            arguments,
            actual: Vec::new(),
            transfers: 0,
            reservation: Some(reservation),
            actions,
        });
    }

    pub(super) fn transfer_call(
        &mut self,
        index: usize,
        value: raw::ValueId,
        owner: raw::PlaceId,
        ty: Ty,
    ) -> OwnerDelta {
        assert!(self.cleanups.is_empty(), "call transfer precedes cleanup");
        let call = self.calls.open.last_mut().expect("call transfer has a scope");
        assert_eq!(call.depth, self.open.len(), "call transfer constructor depth");
        assert_eq!(call.signature.result, ty, "call transfer exact type");
        assert_eq!(call.actual, call.arguments, "call ordered immediate argument results");
        assert_eq!(call.arguments.get(call.transfers), Some(&value), "call ordered transfer value");
        assert_eq!(
            index + call.arguments.len() - call.transfers + 3,
            call.end,
            "call transfer tail range"
        );
        assert_eq!(self.lowerer.owners.owner(value), Some(owner), "call actual argument owner");
        let actual_ty = self.lowerer.places.get(owner.0 as usize).expect("call owner place").ty;
        assert_eq!(actual_ty, ty.ir, "call actual argument type");
        let delta = self.lowerer.owners.transfer(value).expect("prepared available call argument");
        assert_eq!(delta, OwnerDelta::Transferred { owner }, "call transferred owner linkage");
        self.lowerer.preparation_facts.apply(delta);
        call.transfers += 1;
        delta
    }

    pub(super) fn release_call(&mut self, index: usize, ty: Ty) {
        assert!(self.cleanups.is_empty(), "call release precedes cleanup");
        assert!(self.calls.released.is_none(), "one released call");
        let mut call = self.calls.open.pop().expect("call release owns reservation");
        assert_eq!(
            (call.end, call.signature.result, call.depth),
            (index + 3, ty, self.open.len()),
            "call release exact range type and parent"
        );
        assert_eq!(call.actual, call.arguments, "call complete ordered arguments");
        assert_eq!(call.transfers, call.arguments.len(), "call complete argument transfer");
        self.lowerer.preparation_facts.held_cleanup =
            super::super::super::super::owned_lowering_resources::CleanupUsage::release(
                self.lowerer.preparation_facts.held_cleanup,
                call.actions,
            );
        call.reservation.take().expect("one call reservation").release(self.lowerer);
        self.calls.released = Some(call);
    }

    pub(super) fn commit_call(
        &mut self,
        index: usize,
        signature: CallSignature,
        arguments: Vec<raw::ValueId>,
        cleanup: raw::CleanupPlanId,
        (ty, at): (Ty, Span),
    ) -> super::super::super::state::Emission {
        let call = self.calls.released.take().expect("call commit owns released scope");
        assert_eq!(
            (call.end, call.signature, call.depth),
            (index + 1, signature, self.open.len()),
            "call exact released contract"
        );
        assert_eq!(signature.result, ty, "call result exact type");
        assert_eq!(call.arguments, arguments, "call committed ordered operands");
        assert_eq!(self.cleanups, [(cleanup, None)], "call cleanup linkage");
        self.cleanups.clear();
        let emission = self
            .lowerer
            .emit_recorded(
                ty,
                at,
                raw::InstructionKind::DirectCall {
                    callee: signature.id,
                    arguments: arguments.into_iter().map(raw::CallArgument::Value).collect(),
                    cleanup,
                },
            )
            .expect("prepared call result");
        for delta in &emission.owners {
            self.lowerer.preparation_facts.apply(*delta);
        }
        emission
    }
}
