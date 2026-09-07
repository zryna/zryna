use super::{
    BorrowIndex, Errors, OwnershipFlow, PlaceState, PlaceStateKind, VecDeque, VerifiedDropAction,
    VerifiedInstruction, VerifiedLayouts, apply_ownership_instruction, apply_value_transfers,
    consuming_instruction_operands, indexed_borrows, layout_type, normalize_dead_places,
    projection_base, push_pending_owner, raw, sealed_drop_action, terminator_edges,
    transfer_consumed_values, transfer_edge_owners, transfer_return_value,
};

pub(super) fn derive_state_before(
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    borrows: &BorrowIndex,
    target_block: usize,
    target_instruction: usize,
) -> Option<(Vec<PlaceState>, Vec<Option<u32>>)> {
    derive_flow_before(function, layouts, borrows, target_block, target_instruction)
        .map(|flow| (flow.states, flow.variants))
}

pub(super) fn construction_failure_actions(
    instruction: VerifiedInstruction<'_>,
) -> Vec<VerifiedDropAction> {
    let function = instruction.function;
    let flow = derive_flow_before(
        function.function,
        &function.owner.linear32,
        function.borrows(),
        instruction.block_index,
        instruction.instruction_index,
    )
    .expect("verified constructor has a complete live-owner snapshot");
    flow.pending
        .iter()
        .rev()
        .map(|place| {
            sealed_drop_action(
                function.id(),
                function.function,
                *place,
                &flow.states,
                &flow.variants,
            )
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn derive_flow_before(
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    borrows: &BorrowIndex,
    target_block: usize,
    target_instruction: usize,
) -> Option<OwnershipFlow> {
    let value_count = function
        .parameters
        .iter()
        .chain(function.blocks.iter().flat_map(|block| &block.parameters))
        .chain(
            function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .filter_map(|instruction| instruction.result.as_ref()),
        )
        .map(|value| value.id.0 as usize + 1)
        .max()
        .unwrap_or(0);
    let mut owners = vec![None; value_count];
    for place in &function.places {
        let value = match place.kind {
            raw::PlaceKind::Parameter(index) => {
                function.parameters.get(index as usize).map(|value| value.id)
            }
            raw::PlaceKind::Temporary(value) => Some(value),
            _ => None,
        };
        if let Some(value) = value
            && layout_type(layouts, place.ty).is_some_and(|ty| ty.drop_kind() != 0)
        {
            owners[value.0 as usize] = Some(place.id);
        }
    }
    let mut initial = function
        .places
        .iter()
        .map(|place| PlaceState {
            kind: if matches!(place.kind, raw::PlaceKind::Parameter(_)) {
                PlaceStateKind::Initialized
            } else {
                PlaceStateKind::Uninitialized
            },
        })
        .collect::<Vec<_>>();
    for index in 0..function.places.len() {
        if let Some(base) = projection_base(&function.places[index].kind)
            && initial[base.0 as usize].kind == PlaceStateKind::Initialized
        {
            initial[index] = PlaceState { kind: PlaceStateKind::Initialized };
        }
    }
    let pending = function
        .parameters
        .iter()
        .filter_map(|parameter| owners.get(parameter.id.0 as usize).copied().flatten())
        .collect();
    let mut entries = vec![None; function.blocks.len()];
    entries[0] = Some(OwnershipFlow {
        states: initial,
        variants: vec![None; function.places.len()],
        pending,
    });
    let mut queue = VecDeque::from([0usize]);
    while let Some(block_index) = queue.pop_front() {
        let mut flow = entries[block_index].clone()?;
        let block = &function.blocks[block_index];
        let mut active = borrows.entry_active(function, block_index);
        let mut replay_errors = Errors::default();
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            if block_index == target_block && instruction_index == target_instruction {
                if matches!(instruction.kind, raw::InstructionKind::DirectCall { .. }) {
                    transfer_consumed_values(
                        &consuming_instruction_operands(&instruction.kind),
                        &owners,
                        function,
                        &mut flow,
                        instruction.span,
                        &mut replay_errors,
                    );
                    indexed_borrows::invalidate_call_variants(
                        instruction,
                        function,
                        borrows,
                        &mut flow.variants,
                    );
                }
                return Some(flow);
            }
            let call = matches!(instruction.kind, raw::InstructionKind::DirectCall { .. });
            if call {
                indexed_borrows::invalidate_call_variants(
                    instruction,
                    function,
                    borrows,
                    &mut flow.variants,
                );
                transfer_consumed_values(
                    &consuming_instruction_operands(&instruction.kind),
                    &owners,
                    function,
                    &mut flow,
                    instruction.span,
                    &mut replay_errors,
                );
            }
            apply_ownership_instruction(
                instruction,
                function,
                (layouts, borrows),
                &mut flow.states,
                &mut flow.variants,
                &mut active,
                &mut replay_errors,
            );
            if !call {
                apply_value_transfers(
                    instruction,
                    &owners,
                    function,
                    layouts,
                    &mut flow,
                    &mut replay_errors,
                );
            }
            if let Some(result) = instruction.result
                && let Some(owner) = owners.get(result.id.0 as usize).copied().flatten()
            {
                if !matches!(
                    instruction.kind,
                    raw::InstructionKind::MoveFromPlace { .. }
                        | raw::InstructionKind::GenericMoveFromPlace { .. }
                ) {
                    push_pending_owner(
                        owner,
                        function,
                        &mut flow,
                        instruction.span,
                        &mut replay_errors,
                    );
                }
                match instruction.kind {
                    raw::InstructionKind::EnumConstruct { variant, .. } => {
                        flow.variants[owner.0 as usize] = Some(variant);
                    }
                    raw::InstructionKind::ClonePlace { place, .. }
                    | raw::InstructionKind::GenericClonePlace { place, .. }
                    | raw::InstructionKind::HandleAwareClonePlace { place, .. } => {
                        flow.variants[owner.0 as usize] = flow.variants[place.0 as usize];
                    }
                    raw::InstructionKind::MoveFromPlace { .. }
                    | raw::InstructionKind::GenericMoveFromPlace { .. } => {}
                    _ => flow.variants[owner.0 as usize] = None,
                }
            }
        }
        if block_index == target_block && target_instruction == block.instructions.len() {
            if let Some(raw::SpannedTerminator {
                kind: raw::Terminator::Return { value, .. },
                span,
            }) = block.terminators.first()
            {
                transfer_return_value(
                    *value,
                    &owners,
                    function,
                    layouts,
                    &mut flow,
                    *span,
                    &mut replay_errors,
                );
            }
            return Some(flow);
        }
        let Some(terminator) = block.terminators.first() else { continue };
        for (edge_index, edge) in terminator_edges(&terminator.kind).into_iter().enumerate() {
            let mut incoming = flow.clone();
            if let raw::Terminator::EnumMatch { place, arms } = &terminator.kind {
                incoming.variants[place.0 as usize] = Some(arms[edge_index].variant);
            }
            transfer_edge_owners(
                edge,
                &terminator.kind,
                function,
                &owners,
                &mut incoming,
                terminator.span,
                &mut replay_errors,
            );
            normalize_dead_places(&mut incoming, function, layouts);
            let target = edge.target.0 as usize;
            if entries[target].is_none() {
                entries[target] = Some(incoming);
                queue.push_back(target);
            }
        }
    }
    None
}
