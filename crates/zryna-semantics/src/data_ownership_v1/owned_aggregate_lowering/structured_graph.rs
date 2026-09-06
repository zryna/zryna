use std::collections::BTreeSet;
use std::ops::Range;
use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

use super::super::owned_cfg_state::OwnedCfgState;
use super::PrivateOwnedAggregateLowerer;

#[cfg(test)]
thread_local! {
    static HELD_RESOURCES: std::cell::Cell<[usize; 2]> = const { std::cell::Cell::new([0, 0]) };
}

#[cfg(not(test))]
pub(super) const fn held_resources() -> [usize; 2] {
    [0, 0]
}

#[cfg(test)]
pub(super) fn held_resources() -> [usize; 2] {
    HELD_RESOURCES.get()
}

#[cfg(test)]
pub(super) fn with_held_resources<T>(held: [usize; 2], f: impl FnOnce() -> T) -> T {
    HELD_RESOURCES.with(|resources| {
        let previous = resources.replace(held);
        let result = f();
        resources.set(previous);
        result
    })
}

pub(super) struct StructuredBlock {
    parameters: Vec<raw::ValueDefinition>,
    start: usize,
    range: Range<usize>,
    pub(super) terminator: Option<raw::SpannedTerminator>,
}

pub(super) struct StructuredGraph {
    pub(super) blocks: Vec<StructuredBlock>,
    pub(super) current: usize,
    edges: usize,
    held_blocks: usize,
    held_edges: usize,
    held_terminators: usize,
    matches: BTreeSet<u32>,
}

impl StructuredGraph {
    pub(super) fn new(
        function: &zryna_syntax::v4::RawFunctionSyntax,
        held_blocks: usize,
        held_edges: usize,
        at: Span,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    ) -> Option<Self> {
        if held_blocks.checked_add(1).is_none_or(|blocks| blocks > ir::MAX_BLOCKS_PER_FUNCTION) {
            Self::limit(lowerer, at, "blocks");
            return None;
        }
        if held_edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
            Self::limit(lowerer, at, "edges");
            return None;
        }
        Some(Self {
            blocks: vec![StructuredBlock {
                parameters: Vec::new(),
                start: 0,
                range: 0..0,
                terminator: None,
            }],
            current: 0,
            edges: 0,
            held_blocks,
            held_edges,
            held_terminators: 0,
            matches: function
                .body
                .expressions
                .iter()
                .filter_map(|expression| {
                    matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
                        .then_some(expression.span.start)
                })
                .collect(),
        })
    }

    fn limit(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, at: Span, resource: &str) {
        lowerer.errors.at(
            "ZRYNA-M3201",
            at,
            format!("structured ownership {resource} exceed the checked limit"),
            "reduce structured control flow",
        );
    }

    pub(super) fn contains_match(&self, start: u32, end: u32) -> bool {
        self.matches.range(start..end).next().is_some()
    }

    pub(super) fn next(
        &mut self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        at: Span,
    ) -> Option<raw::BlockId> {
        let blocks = self
            .held_blocks
            .checked_add(self.blocks.len())
            .and_then(|blocks| blocks.checked_add(1));
        if blocks.is_none_or(|blocks| blocks > ir::MAX_BLOCKS_PER_FUNCTION) {
            Self::limit(lowerer, at, "blocks");
            return None;
        }
        let id = raw::BlockId(u32::try_from(self.blocks.len()).ok()?);
        self.current = self.blocks.len();
        let start = lowerer.instructions.len();
        self.blocks.push(StructuredBlock {
            parameters: Vec::new(),
            start,
            range: start..start,
            terminator: None,
        });
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
        let edges =
            self.held_edges.checked_add(self.edges).and_then(|edges| edges.checked_add(additional));
        if edges.is_none_or(|edges| edges > ir::MAX_CFG_EDGES_PER_FUNCTION) {
            Self::limit(lowerer, at, "edges");
            return None;
        }
        if !lowerer.reserve_transition(at) {
            return None;
        }
        self.edges =
            self.edges.checked_add(additional).expect("held total checked generated edges");
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

    pub(super) fn result_parameter(
        &mut self,
        lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
        ty: super::Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        if !lowerer.resource_usage().value(at, lowerer.errors)
            || !lowerer.resource_usage().places(usize::from(!ty.is_copy()), at, lowerer.errors)
        {
            return None;
        }
        let value = raw::ValueId(lowerer.next_value);
        let definition = raw::ValueDefinition { id: value, ty: ty.ir, span: at };
        lowerer
            .constructor_types
            .record_block_parameter(&lowerer.instructions, &definition)
            .ok()?;
        lowerer.next_value += 1;
        if !ty.is_copy() {
            let owner = raw::PlaceId(u32::try_from(lowerer.places.len()).ok()?);
            lowerer.places.push(raw::Place {
                id: owner,
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Temporary(value),
            });
            lowerer.owners.register(value, owner)?;
        }
        self.blocks[self.current].parameters.push(definition);
        Some(value)
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
                    block.parameters,
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
