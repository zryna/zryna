//! Each attack changes a concrete checked instruction role in a genuine fixture artifact.
use super::{I, Inst, O, model, replace};
#[derive(Clone, Copy, Debug)]
pub(super) enum Attack {
    UnclassifiedGuard,
    EarlyTrap,
    RuntimeTarget,
    OutputOffset,
    InvertedStatus,
    UnconditionalCleanup,
    EdgeTarget,
    IndexSource,
    BranchCondition,
    CopyStall,
    ArrayBound,
    HeapBound,
    BusyReset,
    TraceBound,
    CloneFaultKind,
    ObserverStatus,
}
impl Attack {
    pub(super) fn function(self, m: &model::Model<'_, '_, '_>) -> usize {
        match self {
            Self::CopyStall | Self::UnclassifiedGuard | Self::EarlyTrap => 0,
            Self::HeapBound => 4,
            Self::BusyReset => m.wrapper_base as usize,
            Self::TraceBound => m.recorder as usize,
            Self::CloneFaultKind => 2,
            Self::ObserverStatus => m.observer as usize,
            Self::ArrayBound => {
                5 + m.types.len()
                    + m.types
                        .iter()
                        .position(|t| t.category() == zryna_layout::TypeCategory::FixedArray)
                        .expect("real array type")
            }
            _ => m.source_base as usize + m.program.export_functions()[0],
        }
    }
    pub(super) fn change(self, body: &mut Vec<u8>, ops: &[Inst<'_>]) {
        match self {
            Self::UnclassifiedGuard => {
                let begin = ops[0].range.start;
                let end = ops.last().expect("function end").range.start;
                body.splice(end..end, [0x0b]);
                body.splice(begin..begin, [0x41, 0, 0x04, 0x40]);
            }
            Self::EarlyTrap => {
                let begin = ops[0].range.start;
                body.splice(begin..begin, [0x00]);
            }
            Self::RuntimeTarget => {
                let i = locate(ops, |o| matches!(o, O::Call { function_index: 1 }));
                replace(body, &ops[i].range, &I::Call(3));
            }
            Self::OutputOffset => {
                let call = locate(ops, |o| matches!(o, O::Call { function_index: 1 }));
                let i = call - 2;
                let O::I32Const { value } = ops[i].op else { panic!("exact frame output address") };
                replace(body, &ops[i].range, &I::I32Const(value + 4));
            }
            Self::InvertedStatus | Self::UnconditionalCleanup => status(self, body, ops),
            Self::EdgeTarget => {
                let set = locate(ops, |o| matches!(o, O::LocalSet { local_index: 2 }));
                let i = set - 1;
                let O::I32Const { value } = ops[i].op else { panic!("edge target constant") };
                replace(body, &ops[i].range, &I::I32Const(value + 1));
            }
            Self::IndexSource => {
                let tee = locate(ops, |o| matches!(o, O::LocalTee { local_index: 5 }));
                let i = tee - 3;
                let O::I32Const { value } = ops[i].op else { panic!("exact index source slot") };
                replace(body, &ops[i].range, &I::I32Const(value + 4));
            }
            Self::BranchCondition => {
                let i = ops
                    .windows(2)
                    .position(|w| {
                        matches!(w[0].op, O::I32Load8U { .. }) && matches!(w[1].op, O::If { .. })
                    })
                    .expect("source bool branch");
                body.splice(ops[i + 1].range.start..ops[i + 1].range.start, [0x45]);
            }
            Self::CopyStall => {
                let i = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].op, O::I32Const { value: 1 })
                            && matches!(w[1].op, O::I32Add)
                            && matches!(w[2].op, O::LocalSet { local_index: 3 })
                    })
                    .expect("actual copy cursor");
                replace(body, &ops[i].range, &I::I32Const(0));
            }
            Self::ArrayBound => {
                let i = locate(ops, |o| matches!(o, O::I32Const { value: 2 }));
                replace(body, &ops[i].range, &I::I32Const(0));
            }
            Self::HeapBound => {
                let i = locate(ops, |o| matches!(o, O::I32Const { value: 16_777_216 }));
                replace(body, &ops[i].range, &I::I32Const(8_388_608));
            }
            Self::BusyReset => {
                let call = locate(ops, |o| matches!(o, O::Call { .. }));
                let i = call + 2;
                assert!(matches!(ops[i].op, O::I32Const { value: 0 }));
                replace(body, &ops[i].range, &I::I32Const(1));
            }
            Self::TraceBound => {
                let i = locate(ops, |o| matches!(o, O::I32Const { value: 4096 }));
                replace(body, &ops[i].range, &I::I32Const(8192));
            }
            Self::CloneFaultKind => {
                let i = locate(ops, |o| matches!(o, O::I32Const { value: 6 }));
                replace(body, &ops[i].range, &I::I32Const(2));
            }
            Self::ObserverStatus => {
                let i = locate(ops, |o| matches!(o, O::GlobalGet { global_index: 1 }));
                replace(body, &ops[i].range, &I::GlobalGet(0));
            }
        }
    }
}
fn locate(ops: &[Inst<'_>], predicate: impl Fn(&O<'_>) -> bool) -> usize {
    ops.iter().position(|i| predicate(&i.op)).expect("exact actual instruction role")
}
fn status(attack: Attack, body: &mut Vec<u8>, ops: &[Inst<'_>]) {
    let i = ops
        .windows(2)
        .position(|w| {
            matches!(w[0].op, O::LocalTee { local_index: 3 }) && matches!(w[1].op, O::If { .. })
        })
        .expect("actual operation status guard")
        + 1;
    if matches!(attack, Attack::InvertedStatus) {
        body.splice(ops[i].range.start..ops[i].range.start, [0x45]);
        return;
    }
    let mut depth = 1;
    let end = (i + 1..ops.len())
        .find(|j| {
            match ops[*j].op {
                O::If { .. } | O::Block { .. } | O::Loop { .. } => depth += 1,
                O::End => depth -= 1,
                _ => {}
            }
            depth == 0
        })
        .expect("matching failure end");
    body.drain(ops[end].range.clone());
    body.splice(ops[i].range.end..ops[i].range.end, [0x0b]);
}
