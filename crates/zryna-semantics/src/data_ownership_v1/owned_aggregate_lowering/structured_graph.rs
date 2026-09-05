use std::ops::Range;
use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

use super::super::owned_cfg_state::OwnedCfgState;
use super::PrivateOwnedAggregateLowerer;

pub(super) struct StructuredBlock {
    start: usize,
    range: Range<usize>,
    pub(super) terminator: Option<raw::SpannedTerminator>,
}

pub(super) struct StructuredGraph {
    pub(super) blocks: Vec<StructuredBlock>,
    pub(super) current: usize,
    edges: usize,
    held_terminators: usize,
}

impl StructuredGraph {
    pub(super) fn new() -> Self {
        Self {
            blocks: vec![StructuredBlock { start: 0, range: 0..0, terminator: None }],
            current: 0,
            edges: 0,
            held_terminators: 0,
        }
    }

    pub(super) fn next(
        &mut self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        at: Span,
    ) -> Option<raw::BlockId> {
        if self.blocks.len() >= ir::MAX_BLOCKS_PER_FUNCTION {
            lowerer.errors.at(
                "ZRYNA-M3201",
                at,
                "structured ownership blocks exceed the checked limit",
                "reduce structured control flow",
            );
            return None;
        }
        let id = raw::BlockId(u32::try_from(self.blocks.len()).ok()?);
        self.current = self.blocks.len();
        let start = lowerer.instructions.len();
        self.blocks.push(StructuredBlock { start, range: start..start, terminator: None });
        Some(id)
    }

    pub(super) fn terminate(
        &mut self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        at: Span,
        kind: raw::Terminator,
    ) -> Option<usize> {
        let additional = match &kind {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            raw::Terminator::EnumMatch { arms, .. } => arms.len(),
        };
        let edges = self.edges.checked_add(additional)?;
        if edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
            lowerer.errors.at(
                "ZRYNA-M3201",
                at,
                "structured ownership edges exceed the checked limit",
                "reduce structured control flow",
            );
            return None;
        }
        if !lowerer.reserve_transition(at) {
            return None;
        }
        self.edges = edges;
        self.held_terminators += 1;
        let index = self.current;
        let block = self.blocks.get_mut(index)?;
        assert!(block.terminator.is_none(), "one structured terminator");
        block.range = block.start..lowerer.instructions.len();
        block.terminator = Some(raw::SpannedTerminator { span: at, kind });
        Some(index)
    }

    pub(super) fn jump(
        &mut self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        at: Span,
        target: raw::BlockId,
    ) -> Option<usize> {
        self.terminate(
            lowerer,
            at,
            raw::Terminator::Jump(raw::Edge { target, arguments: Vec::new() }),
        )
    }

    pub(super) fn retarget_jump(&mut self, block: usize, target: raw::BlockId) {
        let raw::Terminator::Jump(edge) =
            &mut self.blocks[block].terminator.as_mut().expect("jump").kind
        else {
            unreachable!("planned jump")
        };
        edge.target = target;
    }

    pub(super) fn finish(
        self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        parameters: &[raw::ValueDefinition],
        at: Span,
    ) -> Option<Vec<raw::Block>> {
        for _ in 0..self.held_terminators {
            lowerer.release_transition();
        }
        let mut cfg = OwnedCfgState::single_block(at, lowerer.errors)?;
        for parameter in parameters {
            cfg.seed_function_parameter(parameter, lowerer.errors)?;
        }
        for _ in 1..self.blocks.len() {
            cfg.reserve_block(at, lowerer.errors)?;
        }
        for (index, block) in self.blocks.into_iter().enumerate() {
            if index != 0 {
                cfg.begin_block(
                    raw::BlockId(u32::try_from(index).ok()?),
                    Vec::new(),
                    at,
                    lowerer.errors,
                )?;
            }
            for instruction in &lowerer.instructions[block.range] {
                if !cfg.emit(instruction.clone(), lowerer.errors) {
                    return None;
                }
            }
            if !cfg.terminate(block.terminator?, lowerer.errors) {
                return None;
            }
        }
        cfg.finish_with_layouts(Some((lowerer.layouts, &lowerer.places)), at, lowerer.errors)
    }
}
