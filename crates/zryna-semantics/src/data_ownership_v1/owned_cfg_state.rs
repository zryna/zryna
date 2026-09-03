use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

use super::diagnostics::Errors;
use super::global_resource_limits::resource_budget_violation;
use super::owned_control_flow_resources::{
    OwnedCfgBudgetLimit, dense_owned_value_id, owned_cfg_budget_violation,
    owned_value_budget_violation,
};

pub(super) struct OwnedPendingBlock {
    pub(super) populated: bool,
    pub(super) parameters: Vec<raw::ValueDefinition>,
    pub(super) instructions: Vec<raw::Instruction>,
    pub(super) terminator: Option<raw::SpannedTerminator>,
}

pub(super) struct OwnedBlockArena {
    pub(super) blocks: Vec<OwnedPendingBlock>,
}

impl OwnedBlockArena {
    fn empty() -> Self {
        Self { blocks: Vec::new() }
    }

    fn finish(self) -> Option<Vec<raw::Block>> {
        self.blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| {
                if !block.populated {
                    return None;
                }
                Some(raw::Block {
                    id: raw::BlockId(u32::try_from(index).ok()?),
                    parameters: block.parameters,
                    instructions: block.instructions,
                    terminators: vec![block.terminator?],
                })
            })
            .collect()
    }
}

// This ledger enforces per-function storage limits. Program-wide block/edge totals remain a
// separate composition check once owned CFG lowering can finalize more than one function graph.
pub(super) struct OwnedCfgState {
    pub(super) arena: OwnedBlockArena,
    pub(super) current: Option<raw::BlockId>,
    pub(super) incoming: Vec<usize>,
    pub(super) edges: usize,
    pub(super) transitions: usize,
    pub(super) reserved_transitions: usize,
    pub(super) value_types: Vec<raw::TypeId>,
    pub(super) reserved_values: usize,
    pub(super) function_parameters_open: bool,
}

impl OwnedCfgState {
    pub(super) fn single_block(at: Span, errors: &mut Errors<'_>) -> Option<Self> {
        let mut state = Self {
            arena: OwnedBlockArena::empty(),
            current: None,
            incoming: Vec::new(),
            edges: 0,
            transitions: 0,
            reserved_transitions: 0,
            value_types: Vec::new(),
            reserved_values: 0,
            function_parameters_open: true,
        };
        let entry = state.reserve_block(at, errors)?;
        state.begin_block(entry, Vec::new(), at, errors)?;
        Some(state)
    }

    pub(super) fn reserve_block(
        &mut self,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<raw::BlockId> {
        let Some(blocks) = self.arena.blocks.len().checked_add(1) else {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return None;
        };
        if owned_cfg_budget_violation(blocks, self.edges, self.transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return None;
        }
        let id = raw::BlockId(u32::try_from(self.arena.blocks.len()).ok()?);
        self.arena.blocks.push(OwnedPendingBlock {
            populated: false,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
        });
        self.incoming.push(0);
        Some(id)
    }

    pub(super) fn preflight_skeleton(
        &self,
        additional_blocks: usize,
        additional_edges: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            self.arena.blocks.len(),
            additional_blocks,
            ir::MAX_BLOCKS_PER_FUNCTION,
        ) {
            Self::limit(OwnedCfgBudgetLimit::Blocks, at, errors);
            return false;
        }
        if resource_budget_violation(self.edges, additional_edges, ir::MAX_CFG_EDGES_PER_FUNCTION) {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        }
        true
    }

    pub(super) fn seed_function_parameter(
        &mut self,
        parameter: &raw::ValueDefinition,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if !self.function_parameters_open {
            Self::shape_error(
                parameter.span,
                "owned CFG function parameters must precede every emitted instruction",
                errors,
            );
            return None;
        }
        let types = self.prevalidate_value_definitions(std::slice::from_ref(parameter), errors)?;
        self.value_types.extend(types);
        Some(())
    }

    pub(super) fn begin_block(
        &mut self,
        id: raw::BlockId,
        parameters: Vec<raw::ValueDefinition>,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if self.current_block().is_some_and(|block| block.terminator.is_none()) {
            Self::shape_error(
                at,
                "cannot select another owned CFG block before terminating the current block",
                errors,
            );
            return None;
        }
        let Some(index) = usize::try_from(id.0).ok() else {
            Self::shape_error(at, "owned CFG block identity is not representable", errors);
            return None;
        };
        let next = self.arena.blocks.iter().position(|block| !block.populated);
        let Some(block) = self.arena.blocks.get(index) else {
            Self::shape_error(at, "owned CFG selected an unreserved block identity", errors);
            return None;
        };
        if block.populated || next != Some(index) {
            Self::shape_error(
                at,
                "owned CFG blocks must be populated once in canonical reservation order",
                errors,
            );
            return None;
        }
        let types = self.prevalidate_value_definitions(&parameters, errors)?;
        let block = self.arena.blocks.get_mut(index).expect("reserved block checked");
        block.populated = true;
        block.parameters = parameters;
        self.current = Some(id);
        self.value_types.extend(types);
        if id.0 != 0 {
            self.function_parameters_open = false;
        }
        Some(())
    }

    fn prevalidate_value_definitions(
        &mut self,
        definitions: &[raw::ValueDefinition],
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::TypeId>> {
        let Some(capacity_count) = self.value_types.len().checked_add(self.reserved_values) else {
            let at = definitions.first()?.span;
            Self::limit(OwnedCfgBudgetLimit::Values, at, errors);
            return None;
        };
        if owned_value_budget_violation(capacity_count, definitions.len()) {
            let trigger = ir::MAX_VALUES_PER_FUNCTION
                .checked_sub(capacity_count)
                .and_then(|remaining| definitions.get(remaining))
                .or_else(|| definitions.first())?;
            Self::limit(OwnedCfgBudgetLimit::Values, trigger.span, errors);
            return None;
        }
        let mut count = self.value_types.len();
        let mut types = Vec::with_capacity(definitions.len());
        for definition in definitions {
            let Some(expected) = dense_owned_value_id(count) else {
                Self::limit(OwnedCfgBudgetLimit::Values, definition.span, errors);
                return None;
            };
            if definition.id != expected {
                Self::shape_error(
                    definition.span,
                    "owned CFG value definitions break dense global value order",
                    errors,
                );
                return None;
            }
            count = count.checked_add(1)?;
            types.push(definition.ty);
        }
        Some(types)
    }

    pub(super) fn reserve_values(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        let current = self.value_types.len().checked_add(self.reserved_values);
        if current.is_none_or(|current| owned_value_budget_violation(current, additional)) {
            Self::limit(OwnedCfgBudgetLimit::Values, at, errors);
            return None;
        }
        self.reserved_values = self.reserved_values.checked_add(additional)?;
        Some(())
    }

    pub(super) fn release_values(&mut self, additional: usize) {
        self.reserved_values =
            self.reserved_values.checked_sub(additional).expect("reserved owned CFG values");
    }

    pub(super) fn current_block(&self) -> Option<&OwnedPendingBlock> {
        self.current
            .and_then(|id| usize::try_from(id.0).ok())
            .and_then(|index| self.arena.blocks.get(index))
    }

    fn current_mut(&mut self) -> Option<&mut OwnedPendingBlock> {
        self.current
            .and_then(|id| usize::try_from(id.0).ok())
            .and_then(|index| self.arena.blocks.get_mut(index))
    }

    pub(super) fn emit(&mut self, instruction: raw::Instruction, errors: &mut Errors<'_>) -> bool {
        if !self.preflight_emit(&instruction, errors) {
            return false;
        }
        let transitions = self.transitions + 1;
        let result_type = instruction.result.as_ref().map(|result| result.ty);
        self.current_mut().expect("current block checked").instructions.push(instruction);
        self.transitions = transitions;
        self.value_types.extend(result_type);
        self.function_parameters_open = false;
        true
    }

    pub(super) fn preflight_transition(&mut self, at: Span, errors: &mut Errors<'_>) -> bool {
        self.preflight_transitions(1, at, errors)
    }

    pub(super) fn preflight_transitions(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if self.current_mut().is_none_or(|block| block.terminator.is_some()) {
            Self::shape_error(
                at,
                "owned CFG emission requires one selected unterminated block",
                errors,
            );
            return false;
        }
        let Some(transitions) = self
            .transitions
            .checked_add(self.reserved_transitions)
            .and_then(|current| current.checked_add(additional))
        else {
            Self::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        };
        if owned_cfg_budget_violation(self.arena.blocks.len(), self.edges, transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        }
        true
    }

    pub(super) fn reserve_transitions(
        &mut self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        if !self.preflight_transitions(additional, at, errors) {
            return None;
        }
        self.reserved_transitions = self.reserved_transitions.checked_add(additional)?;
        Some(())
    }

    pub(super) fn release_transitions(&mut self, additional: usize) {
        self.reserved_transitions = self
            .reserved_transitions
            .checked_sub(additional)
            .expect("reserved owned CFG transitions");
    }

    pub(super) fn preflight_emit(
        &mut self,
        instruction: &raw::Instruction,
        errors: &mut Errors<'_>,
    ) -> bool {
        let at = instruction.span;
        if !self.preflight_transition(at, errors) {
            return false;
        }
        if let Some(result) = &instruction.result {
            self.prevalidate_value_definitions(std::slice::from_ref(result), errors).is_some()
        } else {
            true
        }
    }

    pub(super) fn terminate(
        &mut self,
        terminator: raw::SpannedTerminator,
        errors: &mut Errors<'_>,
    ) -> bool {
        let at = terminator.span;
        if self.current_mut().is_none_or(|block| block.terminator.is_some()) {
            Self::shape_error(
                at,
                "owned CFG termination requires one selected unterminated block",
                errors,
            );
            return false;
        }
        let Some(targets) = self.validate_targets(&terminator.kind, at, errors) else {
            return false;
        };
        let additional = match &terminator.kind {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
            raw::Terminator::Jump(_) => 1,
            raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
            raw::Terminator::EnumMatch { arms, .. } => arms.len(),
        };
        let Some(edges) = self.edges.checked_add(additional) else {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        };
        if owned_cfg_budget_violation(self.arena.blocks.len(), edges, self.transitions).is_some() {
            Self::limit(OwnedCfgBudgetLimit::Edges, at, errors);
            return false;
        }
        for target in targets {
            let index = usize::try_from(target.0).expect("reserved target index");
            self.incoming[index] = self.incoming[index].saturating_add(1);
        }
        self.current_mut().expect("current block checked").terminator = Some(terminator);
        self.edges = edges;
        true
    }

    fn validate_targets(
        &mut self,
        terminator: &raw::Terminator,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::BlockId>> {
        let targets = match terminator {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
            raw::Terminator::Jump(edge) => vec![edge.target],
            raw::Terminator::Branch { when_true, when_false, .. } => {
                vec![when_true.target, when_false.target]
            }
            raw::Terminator::EnumMatch { arms, .. } => {
                arms.iter().map(|arm| arm.edge.target).collect()
            }
            raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                vec![success.target, expired.target]
            }
        };
        let Some(current) = self.current else {
            Self::shape_error(at, "owned CFG has no current block for its terminator", errors);
            return None;
        };
        for target in targets {
            let reserved = usize::try_from(target.0)
                .ok()
                .is_some_and(|index| self.arena.blocks.get(index).is_some());
            if !reserved || target.0 == 0 {
                Self::shape_error(
                    at,
                    "owned CFG successor must be a reserved non-entry block",
                    errors,
                );
                return None;
            }
        }
        let _ = current;
        Some(match terminator {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
            raw::Terminator::Jump(edge) => vec![edge.target],
            raw::Terminator::Branch { when_true, when_false, .. } => {
                vec![when_true.target, when_false.target]
            }
            raw::Terminator::EnumMatch { arms, .. } => {
                arms.iter().map(|arm| arm.edge.target).collect()
            }
            raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                vec![success.target, expired.target]
            }
        })
    }

    fn limit(limit: OwnedCfgBudgetLimit, at: Span, errors: &mut Errors<'_>) {
        let (label, maximum, guidance) = match limit {
            OwnedCfgBudgetLimit::Blocks => (
                "owned CFG blocks",
                ir::MAX_BLOCKS_PER_FUNCTION,
                "reduce nested owned control-flow blocks",
            ),
            OwnedCfgBudgetLimit::Edges => (
                "owned CFG edges",
                ir::MAX_CFG_EDGES_PER_FUNCTION,
                "reduce owned branch and loop edges",
            ),
            OwnedCfgBudgetLimit::Transitions => (
                "owned CFG transitions",
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                "reduce owned operations before control-flow lowering",
            ),
            OwnedCfgBudgetLimit::Values => (
                "owned CFG values",
                ir::MAX_VALUES_PER_FUNCTION,
                "reduce owned function parameters, block parameters, and result-producing expressions",
            ),
        };
        errors.at(
            "ZRYNA-M3201",
            at,
            format!("{label} exceed the per-function M3 limit of {maximum}"),
            guidance,
        );
    }

    fn shape_error(at: Span, message: &'static str, errors: &mut Errors<'_>) {
        errors.at(
            "ZRYNA-M3015",
            at,
            message,
            "reserve blocks first, populate them once in order, and terminate each block exactly once",
        );
    }

    pub(super) fn finish(self, at: Span, errors: &mut Errors<'_>) -> Option<Vec<raw::Block>> {
        if self.arena.blocks.is_empty() {
            Self::shape_error(at, "owned CFG has no entry block", errors);
            return None;
        }
        if self.arena.blocks.iter().any(|block| !block.populated) {
            Self::shape_error(at, "owned CFG contains an unpopulated reserved block", errors);
            return None;
        }
        if self.arena.blocks.iter().any(|block| block.terminator.is_none()) {
            Self::shape_error(at, "owned CFG contains an unterminated block", errors);
            return None;
        }
        if self.incoming.iter().skip(1).any(|incoming| *incoming == 0) {
            Self::shape_error(
                at,
                "owned CFG contains a non-entry block with no predecessor",
                errors,
            );
            return None;
        }
        let mut reachable = vec![false; self.arena.blocks.len()];
        reachable[0] = true;
        let mut work = vec![0_usize];
        while let Some(index) = work.pop() {
            let terminator = &self.arena.blocks[index]
                .terminator
                .as_ref()
                .expect("terminated blocks checked")
                .kind;
            let targets = match terminator {
                raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
                raw::Terminator::Jump(edge) => vec![edge.target],
                raw::Terminator::Branch { when_true, when_false, .. } => {
                    vec![when_true.target, when_false.target]
                }
                raw::Terminator::EnumMatch { arms, .. } => {
                    arms.iter().map(|arm| arm.edge.target).collect()
                }
                raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                    vec![success.target, expired.target]
                }
            };
            for target in targets {
                let target = usize::try_from(target.0).expect("reserved target index");
                if !reachable[target] {
                    reachable[target] = true;
                    work.push(target);
                }
            }
        }
        if reachable.iter().any(|reachable| !reachable) {
            Self::shape_error(at, "owned CFG contains blocks disconnected from its entry", errors);
            return None;
        }
        for block in &self.arena.blocks {
            let terminator = block.terminator.as_ref().expect("terminated blocks checked");
            let edges = match &terminator.kind {
                raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Vec::new(),
                raw::Terminator::Jump(edge) => vec![edge],
                raw::Terminator::Branch { when_true, when_false, .. } => {
                    vec![when_true, when_false]
                }
                raw::Terminator::EnumMatch { arms, .. } => {
                    arms.iter().map(|arm| &arm.edge).collect()
                }
                raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
                    vec![success, expired]
                }
            };
            for edge in edges {
                let target = &self.arena.blocks
                    [usize::try_from(edge.target.0).expect("reserved target index")];
                if edge.arguments.len() != target.parameters.len()
                    || edge.arguments.iter().zip(&target.parameters).any(|(argument, parameter)| {
                        usize::try_from(argument.0)
                            .ok()
                            .and_then(|index| self.value_types.get(index))
                            != Some(&parameter.ty)
                    })
                {
                    Self::shape_error(
                        at,
                        "owned CFG edge arguments do not match the populated target signature",
                        errors,
                    );
                    return None;
                }
            }
        }
        Some(self.arena.finish().expect("populated dense blocks checked"))
    }
}

pub(super) fn reserve_owned_commit_transition(
    cfg: &mut OwnedCfgState,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    reserve_owned_commit_transitions(cfg, 1, at, errors)
}

pub(super) fn release_owned_commit_transition(cfg: &mut OwnedCfgState) {
    release_owned_commit_transitions(cfg, 1);
}

pub(super) fn reserve_owned_commit_transitions(
    cfg: &mut OwnedCfgState,
    transitions: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    cfg.reserve_transitions(transitions, at, errors).is_some()
}

pub(super) fn release_owned_commit_transitions(cfg: &mut OwnedCfgState, transitions: usize) {
    cfg.release_transitions(transitions);
}
