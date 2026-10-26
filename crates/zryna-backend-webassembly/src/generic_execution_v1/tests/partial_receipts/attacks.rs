//! Hostiles mutate actual source body bytes and their supplemental copies, never retained proofs.
use super::{Encode, I, O, model, raw};
use std::ops::Range;
#[derive(Clone, Copy, Debug)]
pub(super) enum Attack {
    Omitted,
    TargetType,
    TargetAddress,
    EarlyCommit,
    EmptyWholeDrop,
}
impl Attack {
    pub(super) const fn case(self) -> &'static str {
        if matches!(self, Self::EmptyWholeDrop) {
            "whole-vs-fields"
        } else {
            "partial-root-replacement"
        }
    }
    pub(super) fn change(
        self,
        ops: &[(O<'_>, Range<usize>)],
        m: &model::Model<'_, '_, '_>,
        f: usize,
        bytes: &mut Vec<u8>,
    ) {
        let op = m.functions[f]
            .blocks()
            .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::operations)
            .find(|op| op.drop_targets().is_some_and(|r| !r.is_whole()))
            .expect("actual same-operation nonwhole receipt");
        let (raw::Operation::ReplacePlace { place: selected, .. }
        | raw::Operation::DropPlace { place: selected }) = *op.kind()
        else {
            unreachable!()
        };
        let root = &m.memory.claims().functions[f].places[selected as usize];
        let raw::ValueType::Stored(root_ty) = root.ty else { unreachable!() };
        if matches!(self, Self::EmptyWholeDrop) {
            assert_eq!(op.drop_targets().expect("retained receipt").targets().len(), 0);
            let block = i32::try_from(op.site().block).expect("actual bounded source block");
            let at = ops
                .windows(4)
                .position(|w| {
                    matches!(w[0].0, O::LocalGet { local_index: 2 })
                        && matches!(w[1].0, O::I32Const { value } if value == block)
                        && matches!(w[2].0, O::I32Eq)
                        && matches!(w[3].0, O::If { .. })
                })
                .expect("actual zero-frontier source block");
            let mut inserted = Vec::new();
            for instruction in [
                I::LocalGet(0),
                I::I32Const(root.offset.cast_signed()),
                I::I32Add,
                I::Call(m.drop_index(root_ty)),
            ] {
                instruction.encode(&mut inserted);
            }
            bytes.splice(ops[at + 3].1.end..ops[at + 3].1.end, inserted);
            return;
        }
        let target = op
            .drop_targets()
            .expect("retained receipt")
            .targets()
            .next()
            .expect("actual initialized sibling");
        let zryna_ir::generic_v1::execution_v1::VerifiedDropTarget::Place(target) = target else {
            unreachable!()
        };
        let target_slot = &m.memory.claims().functions[f].places[target.id() as usize];
        let helper = m.drop_index(target.ty());
        // The final exact sibling call is the old-drop site; earlier equal calls belong
        // to RHS allocation-failure cleanup and must remain in their original guards.
        let at = ops.windows(4).rposition(|w| matches!(w[0].0, O::LocalGet { local_index: 0 })
            && matches!(w[1].0, O::I32Const { value } if value == target_slot.offset.cast_signed())
            && matches!(w[2].0, O::I32Add) && matches!(w[3].0, O::Call { function_index } if function_index == helper))
            .expect("actual exact old sibling drop call");
        if matches!(self, Self::EarlyCommit) {
            let copy = ops
                .iter()
                .enumerate()
                .skip(at + 4)
                .find(|(_, (o, _))| matches!(o, O::Call { function_index: 0 }))
                .expect("prepared RHS commit after drop")
                .0;
            let start = ops[at].1.start;
            let middle = ops[at + 3].1.end;
            let end = ops[copy].1.end;
            let mut swapped = bytes[middle..end].to_vec();
            swapped.extend_from_slice(&bytes[start..middle]);
            bytes.splice(start..end, swapped);
            return;
        }
        let (index, replacement) = match self {
            Self::Omitted => (at + 3, I::Drop),
            Self::TargetType => (at + 3, I::Call(m.drop_index(root_ty))),
            Self::TargetAddress => {
                assert_ne!(root.offset, target_slot.offset);
                (at + 1, I::I32Const(root.offset.cast_signed()))
            }
            Self::EarlyCommit | Self::EmptyWholeDrop => unreachable!(),
        };
        let mut encoded = Vec::new();
        replacement.encode(&mut encoded);
        bytes.splice(ops[index].1.clone(), encoded);
    }
}
