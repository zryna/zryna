use super::{Errors, OwnedBlockArena, OwnedCfgState, Span, raw};

impl OwnedBlockArena {
    pub(super) fn finish(self) -> Option<Vec<raw::Block>> {
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

impl OwnedCfgState {
    pub(in super::super) fn finish(
        self,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::Block>> {
        self.finish_with_layouts(None, at, errors)
    }

    pub(in super::super) fn finish_with_layouts(
        self,
        context: Option<(&zryna_layout::VerifiedLayouts, &[raw::Place])>,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Vec<raw::Block>> {
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
        self.validate_edge_signatures(context, at, errors)?;
        Some(self.arena.finish().expect("populated dense blocks checked"))
    }

    fn validate_edge_signatures(
        &self,
        context: Option<(&zryna_layout::VerifiedLayouts, &[raw::Place])>,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<()> {
        let mut shapes = std::collections::BTreeMap::new();
        for block in &self.arena.blocks {
            let terminator = block.terminator.as_ref().expect("terminated blocks checked");
            let upgrade = if let raw::Terminator::WeakUpgradeBranch { weak, .. } = &terminator.kind
            {
                let shape = context.and_then(|(layouts, places)| {
                    let place = places.get(usize::try_from(weak.0).ok()?)?;
                    if place.id != *weak {
                        return None;
                    }
                    *shapes.entry(place.ty).or_insert_with(|| {
                        let ty = layouts.types().find(|ty| ty.id().index() == place.ty.0)?.id();
                        zryna_ir::data_ownership_v1::WeakUpgradeShape::derive(layouts, ty)
                    })
                });
                let Some(shape) = shape else {
                    Self::shape_error(
                        at,
                        "owned CFG upgrade requires an exact sealed Weak shape",
                        errors,
                    );
                    return None;
                };
                Some(raw::TypeId(shape.success_parameter_type().index()))
            } else {
                None
            };
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
            for (ordinal, edge) in edges.into_iter().enumerate() {
                let target = &self.arena.blocks
                    [usize::try_from(edge.target.0).expect("reserved target index")];
                let parameters = if ordinal == 0
                    && let Some(shared) = upgrade
                {
                    let Some((synthetic, ordinary)) = target.parameters.split_first() else {
                        Self::shape_error(
                            at,
                            "owned CFG upgrade success lacks its Shared parameter",
                            errors,
                        );
                        return None;
                    };
                    if synthetic.ty != shared {
                        Self::shape_error(
                            at,
                            "owned CFG upgrade success has the wrong Shared parameter",
                            errors,
                        );
                        return None;
                    }
                    ordinary
                } else {
                    &target.parameters
                };
                if edge.arguments.len() != parameters.len()
                    || edge.arguments.iter().zip(parameters).any(|(argument, parameter)| {
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
        Some(())
    }
}
