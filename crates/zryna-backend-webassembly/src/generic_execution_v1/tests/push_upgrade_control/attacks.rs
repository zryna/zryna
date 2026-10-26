//! Small concrete valid-Wasm corruptions, located by roles rather than emitted byte strings.
use super::{I, O, model};
use crate::generic_execution_v1::model::containers as C;
use std::ops::Range;
#[derive(Clone, Copy, Debug)]
pub(super) enum Attack {
    PendingZero,
    GrowthFactor,
    CopySource,
    PendingCapacity,
    ExpiredResult,
    StrongIncrement,
    LiveGuard,
    SuccessEdge,
}
impl Attack {
    pub(super) const fn case(self) -> &'static str {
        match self {
            Self::PendingZero | Self::GrowthFactor | Self::CopySource | Self::PendingCapacity => {
                "vec-push"
            }
            _ => "weak-live",
        }
    }
    pub(super) fn function(self, m: &model::Model<'_, '_, '_>) -> usize {
        match self {
            Self::GrowthFactor | Self::CopySource | Self::PendingCapacity => {
                m.container(C::VEC_RESERVE) as usize
            }
            Self::ExpiredResult | Self::StrongIncrement => m.container(C::WEAK_UPGRADE) as usize,
            _ => m.source_base as usize + m.program.export_functions()[0],
        }
    }
    pub(super) fn change(
        self,
        ops: &[(O<'_>, Range<usize>)],
        m: &model::Model<'_, '_, '_>,
    ) -> (Range<usize>, I<'static>) {
        let index = match self {
            Self::PendingZero => ops
                .windows(2)
                .position(|w| {
                    matches!(w[0].0, O::I32Const { value: 0 })
                        && matches!(w[1].0,O::I32Store { memarg } if memarg.offset==0)
                })
                .expect("actual pending zero"),
            Self::GrowthFactor => ops
                .windows(2)
                .position(|w| {
                    matches!(w[0].0, O::I32Const { value: 2 }) && matches!(w[1].0, O::I32Mul)
                })
                .expect("canonical growth factor"),
            Self::CopySource => ops
                .windows(6)
                .position(|w| {
                    matches!(w[0].0, O::LocalGet { local_index: 4 })
                        && matches!(w[1].0, O::LocalGet { local_index: 8 })
                        && matches!(w[5].0, O::Call { function_index: 0 })
                })
                .expect("actual old initialized storage copy"),
            Self::PendingCapacity => {
                ops.windows(3)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 3 })
                            && matches!(w[1].0, O::LocalGet { local_index: 7 })
                            && matches!(w[2].0,O::I32Store { memarg } if memarg.offset==8)
                    })
                    .expect("actual pending capacity publication")
                    + 2
            }
            Self::ExpiredResult => ops
                .windows(2)
                .position(|w| {
                    matches!(w[0].0, O::I32Const { value: 5 }) && matches!(w[1].0, O::Return)
                })
                .expect("expired result"),
            Self::StrongIncrement => ops
                .windows(3)
                .position(|w| {
                    matches!(w[0].0, O::I32Const { value: 1 })
                        && matches!(w[1].0, O::I32Add)
                        && matches!(w[2].0,O::I32Store { memarg } if memarg.offset==0)
                })
                .expect("live strong increment"),
            Self::LiveGuard => {
                ops.windows(3)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 3 })
                            && matches!(w[1].0, O::I32Eqz)
                            && matches!(w[2].0, O::If { .. })
                    })
                    .expect("exact live status branch")
                    + 1
            }
            Self::SuccessEdge => ops
                .windows(3)
                .position(|w| {
                    matches!(w[0].0, O::I32Const { .. })
                        && matches!(w[1].0, O::LocalSet { local_index: 2 })
                        && matches!(w[2].0, O::Br { relative_depth: 2 })
                })
                .expect("exact successful upgrade edge"),
        };
        let replacement = match self {
            Self::PendingZero => I::I32Const(1),
            Self::GrowthFactor => I::I32Const(3),
            Self::CopySource => I::LocalGet(8),
            Self::PendingCapacity => {
                I::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 })
            }
            Self::ExpiredResult | Self::StrongIncrement => I::I32Const(0),
            Self::LiveGuard => I::I32Clz,
            Self::SuccessEdge => {
                let block = m.functions[m.program.export_functions()[0]]
                    .blocks()
                    .find_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::weak_upgrade)
                    .expect("genuine upgrade terminator")
                    .expired()
                    .target()
                    .id();
                I::I32Const(
                    crate::generic_execution_v1::checked_u32(block)
                        .expect("bounded actual block")
                        .cast_signed(),
                )
            }
        };
        (ops[index].1.clone(), replacement)
    }
}
