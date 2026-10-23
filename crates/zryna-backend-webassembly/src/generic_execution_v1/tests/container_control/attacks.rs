//! Single concrete valid-Wasm mutations of genuine retained container roles.
use super::{I, Inst, O, model, replace};
use crate::generic_execution_v1::model::containers as C;
use wasm_encoder::Encode;
use zryna_layout::TypeCategory;
#[derive(Clone, Copy, Debug)]
pub(super) enum Attack {
    Cursor,
    Stride,
    CountTarget,
    LastStrong,
    PackedStatus,
    LastStrongGuardEscape,
    LiveCountDecrement,
    ValidationBoundary,
    ReturnStatus,
    ControlPredicate,
    PackedCallResult,
}
impl Attack {
    pub(super) fn function(self, m: &model::Model<'_, '_, '_>) -> usize {
        let category = match self {
            Self::PackedCallResult => return m.container(C::VEC_ALLOCATE) as usize,
            Self::ControlPredicate => return m.container(C::STRONG_CLONE) as usize,
            Self::LastStrongGuardEscape => return m.container(C::STRONG_BEGIN) as usize,
            Self::LiveCountDecrement | Self::ValidationBoundary | Self::ReturnStatus => {
                return m.container(C::RELEASE_RAW) as usize;
            }
            Self::Cursor | Self::Stride => TypeCategory::Vec,
            Self::CountTarget | Self::LastStrong => TypeCategory::Shared,
            Self::PackedStatus => return m.container(C::ALLOCATE_RAW) as usize,
        };
        let ty =
            m.types.iter().position(|t| t.category() == category).expect("genuine container type");
        let ty = crate::generic_execution_v1::checked_u32(ty)
            .expect("genuine container type index fits retained u32 authority");
        if matches!(self, Self::LastStrong) {
            m.drop_index(ty) as usize
        } else {
            m.clone_index(ty) as usize
        }
    }
    pub(super) fn change(self, body: &mut Vec<u8>, ops: &[Inst<'_>]) {
        match self {
            Self::PackedCallResult => {
                let index = ops
                    .windows(2)
                    .position(|w| {
                        matches!(w[0].op, O::Call { .. }) && matches!(w[1].op, O::I64Store { .. })
                    })
                    .expect("actual canonical packed allocation call and scratch store");
                let mut extra = Vec::new();
                I::I64Const(1).encode(&mut extra);
                I::I64Or.encode(&mut extra);
                let start = ops[index].range.end;
                body.splice(start..start, extra);
            }
            Self::ControlPredicate
            | Self::ValidationBoundary
            | Self::ReturnStatus
            | Self::LastStrongGuardEscape
            | Self::LiveCountDecrement => {
                self.runtime_controls(body, ops);
            }
            Self::Cursor => {
                let i = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op, O::I32Const { value: 1 })
                            && matches!(w[1].op, O::I32Add)
                            && matches!(w[2].op, O::LocalSet { local_index: 3 })
                    })
                    .expect("actual Vec progress");
                replace(body, &ops[i].range, &I::I32Const(0));
            }
            Self::Stride => {
                let i = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op, O::LocalGet { local_index: 3 })
                            && matches!(w[1].op, O::I32Const { .. })
                            && matches!(w[2].op, O::I32Mul)
                    })
                    .expect("exact dynamic child stride")
                    + 1;
                let O::I32Const { value } = ops[i].op else { panic!("fixed stride") };
                replace(body, &ops[i].range, &I::I32Const(value + 4));
            }
            Self::CountTarget => {
                let i = ops
                    .iter()
                    .position(|v| matches!(v.op, O::Call { .. }))
                    .expect("count terminal");
                let O::Call { function_index } = ops[i].op else { panic!("actual count call") };
                replace(body, &ops[i].range, &I::Call(function_index + 2));
            }
            Self::LastStrong => {
                let i = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op, O::LocalSet { local_index: 1 })
                            && matches!(w[1].op, O::LocalGet { local_index: 1 })
                            && matches!(w[2].op, O::If { .. })
                    })
                    .expect("copied last-strong result")
                    + 2;
                body.splice(ops[i].range.start..ops[i].range.start, [0x45]);
            }
            Self::PackedStatus => {
                let i = ops
                    .iter()
                    .position(|v| matches!(v.op,O::I64Const{value} if value==255i64<<32))
                    .expect("canonical high-status result");
                replace(body, &ops[i].range, &I::I64Const(255));
            }
        }
    }
    fn runtime_controls(self, body: &mut Vec<u8>, ops: &[Inst<'_>]) {
        match self {
            Self::ControlPredicate => {
                let index = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op,O::I32Load { memarg } if memarg.offset==4)
                            && matches!(w[1].op, O::I32Const { value: 8 })
                            && matches!(w[2].op, O::I32LtU)
                    })
                    .expect("exact control size predicate")
                    + 2;
                let start = ops[index].range.end;
                body.splice(start..start, [0x45]);
            }
            Self::ValidationBoundary => {
                let guard = ops
                    .windows(2)
                    .position(|w| matches!(w[0].op, O::I32Ne) && matches!(w[1].op, O::If { .. }))
                    .expect("exact header-live validation")
                    + 1;
                let mut depth = 1usize;
                let end = ((guard + 1)..ops.len())
                    .find(|&i| {
                        match ops[i].op {
                            O::If { .. } | O::Block { .. } | O::Loop { .. } => depth += 1,
                            O::End => depth -= 1,
                            _ => {}
                        }
                        depth == 0
                    })
                    .expect("matching validation boundary");
                body.splice(ops[end].range.clone(), []);
                let start = ops[guard + 1].range.start;
                body.splice(start..start, [0x0b]);
            }
            Self::ReturnStatus => {
                let index = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op, O::GlobalSet { global_index: 1 })
                            && matches!(w[1].op, O::I32Const { value: 255 })
                            && matches!(w[2].op, O::Return)
                    })
                    .expect("canonical runtime failure result")
                    + 1;
                replace(body, &ops[index].range, &I::I32Const(0));
            }
            Self::LastStrongGuardEscape => {
                let i = ops
                    .windows(4)
                    .position(|w| {
                        matches!(w[0].op, O::LocalGet { local_index: 1 })
                            && matches!(w[1].op, O::I32Const { value: 1 })
                            && matches!(w[2].op,O::I32Store{memarg} if memarg.offset==0)
                            && matches!(w[3].op, O::End)
                    })
                    .expect("actual last-strong output publication");
                body.splice(ops[i + 3].range.clone(), []);
                body.splice(ops[i].range.start..ops[i].range.start, [0x0b]);
            }
            Self::LiveCountDecrement => {
                let i = ops
                    .windows(4)
                    .position(|w| {
                        matches!(w[0].op, O::GlobalGet { global_index: 8 })
                            && matches!(w[1].op, O::I32Const { value: 1 })
                            && matches!(w[2].op, O::I32Sub)
                            && matches!(w[3].op, O::GlobalSet { global_index: 8 })
                    })
                    .expect("actual resource-owner decrement")
                    + 1;
                replace(body, &ops[i].range, &I::I32Const(0));
            }
            _ => unreachable!("runtime control helper receives its exact attack group"),
        }
    }
}
