//! Valid changed actual bytes selected from genuine sealed function and helper roles.
use super::{Encode, I, O, model};
use std::ops::Range;
use zryna_ir::generic_v1::execution_v1::{VerifiedParameter, raw};
#[derive(Clone, Copy, Debug)]
pub(super) enum Attack {
    LoanLength,
    LoanCell,
    LoanEnd,
    FormalAddress,
    DropOmitted,
    EarlyCommit,
    FirstCopyLength,
    SecondCopySource,
    SecondDestination,
    CombinedCapacity,
    OutputLength,
}
impl Attack {
    pub(super) const fn case(self) -> &'static str {
        match self {
            Self::LoanLength | Self::LoanCell => "exclusive-reborrow-push",
            Self::LoanEnd | Self::FormalAddress => "formal-copy",
            Self::DropOmitted | Self::EarlyCommit => "generic-replace",
            _ => "local-observation",
        }
    }
    pub(super) fn function(self, m: &model::Model<'_, '_, '_>) -> usize {
        match self {
            Self::LoanLength | Self::LoanCell => {
                m.source_base as usize
                    + m.functions
                        .iter()
                        .position(|f| {
                            f.parameters().any(|p| matches!(p, VerifiedParameter::Loan(_)))
                                && f.blocks()
                                    .any(|b| b.operations().any(|o| o.vec_push().is_some()))
                        })
                        .expect("genuine formal Vec append function")
            }
            Self::LoanEnd | Self::FormalAddress => {
                m.source_base as usize + m.program.export_functions()[0]
            }
            Self::DropOmitted | Self::EarlyCommit => {
                m.source_base as usize
                    + m.functions
                        .iter()
                        .position(|f| {
                            f.parameters().any(|p| matches!(p, VerifiedParameter::Loan(_)))
                                && f.blocks().any(|b| {
                                    b.operations().any(|o| {
                                        matches!(o.kind(), raw::Operation::ReplaceLoan { .. })
                                    })
                                })
                        })
                        .expect("actual closed replacement function")
            }
            _ => m.concat_index() as usize,
        }
    }
    pub(super) fn change(
        self,
        ops: &[(O<'_>, Range<usize>)],
        m: &model::Model<'_, '_, '_>,
        bytes: &mut Vec<u8>,
    ) {
        if matches!(self, Self::EarlyCommit) {
            self.reorder(ops, m, bytes);
            return;
        }
        let (index, replacement) = match self {
            Self::LoanLength | Self::LoanCell => self.loan_load(ops, m),
            Self::LoanEnd => {
                let f = self.function(m) - m.source_base as usize;
                let at = ops.windows(5).position(|w| matches!(w[0].0, O::LocalGet { local_index: 0 })
                    && matches!(w[1].0, O::I32Const { value } if m.memory.claims().functions[f].loans.iter().any(|c| c.offset.cast_signed() == value))
                    && matches!(w[2].0, O::I32Add) && matches!(w[3].0, O::I32Const { value: 0 })
                    && matches!(w[4].0, O::I32Store { .. })).expect("actual own EndLoan clear");
                (at + 3, I::I32Const(1))
            }
            Self::FormalAddress => {
                let at = ops
                    .windows(8)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 4 })
                            && matches!(w[1].0, O::I32Const { .. })
                            && matches!(w[2].0, O::I32Add)
                            && (matches!(w[5].0, O::I32Store { .. })
                                || matches!(w[7].0, O::I32Store { .. }))
                    })
                    .expect("actual formal pointer argument lane");
                let O::I32Const { value } = ops[at + 1].0 else { unreachable!() };
                (at + 1, I::I32Const(value + 4))
            }
            Self::DropOmitted => (self.drop_call(ops, m), I::Drop),
            Self::FirstCopyLength => {
                let at = ops
                    .windows(4)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 3 })
                            && matches!(w[1].0, O::LocalGet { local_index: 10 })
                            && matches!(w[2].0, O::LocalGet { local_index: 4 })
                            && matches!(w[3].0, O::Call { function_index: 0 })
                    })
                    .expect("exact left initialized span");
                (at + 2, I::LocalGet(7))
            }
            Self::SecondCopySource | Self::SecondDestination => {
                let at = ops
                    .windows(6)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 6 })
                            && matches!(w[1].0, O::LocalGet { local_index: 10 })
                            && matches!(w[2].0, O::LocalGet { local_index: 4 })
                            && matches!(w[3].0, O::I32Add)
                            && matches!(w[4].0, O::LocalGet { local_index: 7 })
                            && matches!(w[5].0, O::Call { function_index: 0 })
                    })
                    .expect("separate right source/output suffix");
                if matches!(self, Self::SecondCopySource) {
                    (at, I::LocalGet(3))
                } else {
                    (at + 2, I::LocalGet(7))
                }
            }
            Self::CombinedCapacity => {
                let max = i32::try_from(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES)
                    .expect("canonical maximum");
                let at = ops
                    .iter()
                    .position(|(o, _)| matches!(o, O::I32Const { value } if *value == max))
                    .expect("actual canonical combined boundary");
                (at, I::I32Const(max - 1))
            }
            Self::OutputLength => {
                let at = ops
                    .windows(3)
                    .position(|w| {
                        matches!(w[0].0, O::LocalGet { local_index: 2 })
                            && matches!(w[1].0, O::LocalGet { local_index: 9 })
                            && matches!(w[2].0, O::I32Store { memarg } if memarg.offset == 4)
                    })
                    .expect("exact concat output length publication");
                (at + 2, I::I32Store(wasm_encoder::MemArg { offset: 8, align: 2, memory_index: 0 }))
            }
            Self::EarlyCommit => unreachable!(),
        };
        let mut encoded = Vec::new();
        replacement.encode(&mut encoded);
        bytes.splice(ops[index].1.clone(), encoded);
    }
    fn loan_load(
        self,
        ops: &[(O<'_>, Range<usize>)],
        m: &model::Model<'_, '_, '_>,
    ) -> (usize, I<'static>) {
        let f = self.function(m) - m.source_base as usize;
        let loan = m.functions[f]
            .parameters()
            .find_map(|p| match p {
                VerifiedParameter::Loan(loan) => Some(loan),
                VerifiedParameter::Value(_) => None,
            })
            .expect("actual formal Vec loan");
        let cell = m.loan_offset(f, loan.id()).expect("retained pointer cell");
        let at = ops
            .windows(3)
            .position(|w| {
                matches!(w[0].0, O::I32Const { value: 4 })
                    && matches!(w[1].0, O::I32Add)
                    && matches!(w[2].0, O::I32Load { memarg } if memarg.offset == 0)
            })
            .expect("actual loan-relative Vec length load");
        if matches!(self, Self::LoanLength) {
            (at, I::I32Const(8))
        } else if cell <= 16 {
            assert!(
                matches!(ops[at - 1].0, O::I32Load { memarg } if memarg.offset == u64::from(cell))
            );
            (
                at - 1,
                I::I32Load(wasm_encoder::MemArg {
                    offset: u64::from(cell.checked_sub(4).expect("genuine preceding value slot")),
                    align: 2,
                    memory_index: 0,
                }),
            )
        } else {
            assert!(matches!(ops[at - 3].0, O::I32Const { value } if value == cell.cast_signed()));
            (
                at - 3,
                I::I32Const(
                    cell.checked_sub(4).expect("genuine preceding value slot").cast_signed(),
                ),
            )
        }
    }
    fn drop_call(self, ops: &[(O<'_>, Range<usize>)], m: &model::Model<'_, '_, '_>) -> usize {
        let f = self.function(m) - m.source_base as usize;
        let loan = m.functions[f]
            .parameters()
            .find_map(|p| match p {
                VerifiedParameter::Loan(l) => Some(l),
                VerifiedParameter::Value(_) => None,
            })
            .expect("actual formal target");
        let drop = m.drop_index(loan.referent());
        ops.iter()
            .position(|(op, _)| matches!(op, O::Call { function_index } if *function_index == drop))
            .expect("actual old-value drop call")
    }
    fn reorder(
        self,
        ops: &[(O<'_>, Range<usize>)],
        m: &model::Model<'_, '_, '_>,
        bytes: &mut Vec<u8>,
    ) {
        let f = self.function(m) - m.source_base as usize;
        let loan = m.functions[f]
            .parameters()
            .find_map(|p| match p {
                VerifiedParameter::Loan(l) => Some(l),
                VerifiedParameter::Value(_) => None,
            })
            .expect("actual formal loan");
        let drop = self.drop_call(ops, m);
        let loads =
            if m.loan_offset(f, loan.id()).expect("genuine loan cell") <= 16 { 2 } else { 4 };
        let copy = ops
            .iter()
            .enumerate()
            .skip(drop + 1)
            .find(|(_, (op, _))| matches!(op, O::Call { function_index: 0 }))
            .expect("prepared RHS commit")
            .0;
        let start = ops[drop - loads].1.start;
        let middle = ops[drop].1.end;
        let end = ops[copy].1.end;
        let mut replacement = bytes[middle..end].to_vec();
        replacement.extend_from_slice(&bytes[start..middle]);
        bytes.splice(start..end, replacement);
    }
}
