//! Bounded conformance observations over sealed recipes, never target execution receipts.
use super::*;
use zryna_ir::data_ownership_v1::{
    PlaceIdentity, ValueIdentity, VerifiedBlock, VerifiedFunction, VerifiedHandleAwareClone,
    VerifiedHandleCloneRecipeKind as Kind, VerifiedHandleCloneRecipeNode,
};
use zryna_layout::TypeId as LayoutTypeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PathPart {
    Field(u32),
    Variant(u32),
    Index(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Acquisition {
    // Cleanup uses this path under Claim::destination; source is provenance, never a drop owner.
    pub(super) path: Vec<PathPart>,
    pub(super) operation: LogicalOperation,
    pub(super) source: ValueIdentity,
}

impl Acquisition {
    pub(super) fn cleanup_operation(&self) -> LogicalOperation {
        match self.operation {
            LogicalOperation::VecAllocate => LogicalOperation::VecReleaseStorage,
            LogicalOperation::StringClone => LogicalOperation::StringRelease,
            LogicalOperation::StrongClone => LogicalOperation::StrongReleaseBegin,
            LogicalOperation::WeakClone => LogicalOperation::WeakRelease,
            _ => unreachable!("only sealed clone acquisitions are enumerated"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Claim {
    pub(super) ordinal: usize,
    pub(super) operation: LogicalOperation,
    pub(super) status: RuntimeStatus,
    pub(super) destination: PlaceIdentity,
    pub(super) reverse_prefix: Vec<Acquisition>,
    pub(super) survivors: Vec<PlaceIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Failure {
    Shape,
    Budget,
    Ordinal,
    Operation,
    Status,
    Destination,
    Prefix,
    Survivors,
}

pub(super) fn occurrences(
    function: VerifiedFunction<'_>,
    clone: VerifiedHandleAwareClone<'_>,
    source: ValueIdentity,
) -> Result<Vec<Acquisition>, Failure> {
    let VerifiedHandleAwareCloneSourceAuthority::Root(root) = clone.source_authority() else {
        return Err(Failure::Shape);
    };
    let initializers = function
        .blocks()
        .flat_map(VerifiedBlock::instructions)
        .filter(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.place_operands().collect::<Vec<_>>() == [root]
        })
        .collect::<Vec<_>>();
    if initializers.len() != 1 || initializers[0].value_operands().collect::<Vec<_>>() != [source] {
        return Err(Failure::Shape);
    }
    let nodes = clone.frontier().nodes().collect::<Vec<_>>();
    let mut result = Vec::new();
    visit(function, &nodes, clone.ty(), source, &mut Vec::new(), &mut result)?;
    Ok(result)
}

fn visit(
    function: VerifiedFunction<'_>,
    nodes: &[VerifiedHandleCloneRecipeNode],
    ty: LayoutTypeId,
    value: ValueIdentity,
    path: &mut Vec<PathPart>,
    result: &mut Vec<Acquisition>,
) -> Result<(), Failure> {
    if path.len() > 32 || result.len() >= 64 {
        return Err(Failure::Budget);
    }
    let instruction = function
        .blocks()
        .flat_map(VerifiedBlock::instructions)
        .find(|instruction| instruction.result() == Some(value))
        .ok_or(Failure::Shape)?;
    if instruction.result_type() != Some(ty) {
        return Err(Failure::Shape);
    }
    let node = nodes.iter().find(|node| node.ty() == ty).ok_or(Failure::Shape)?;
    let operands = instruction.value_operands().collect::<Vec<_>>();
    match node.kind() {
        Kind::Copy => {}
        Kind::StringClone | Kind::SharedCountClone | Kind::WeakCountClone => {
            let (kind, operation) = match node.kind() {
                Kind::StringClone => {
                    (VerifiedInstructionKind::StringClone, LogicalOperation::StringClone)
                }
                Kind::SharedCountClone => {
                    (VerifiedInstructionKind::SharedClone, LogicalOperation::StrongClone)
                }
                Kind::WeakCountClone => {
                    (VerifiedInstructionKind::WeakClone, LogicalOperation::WeakClone)
                }
                _ => unreachable!("leaf"),
            };
            if instruction.kind() != kind {
                return Err(Failure::Shape);
            }
            result.push(Acquisition { path: path.clone(), operation, source: value });
        }
        Kind::Struct(fields) => {
            if instruction.kind() != VerifiedInstructionKind::StructConstruct
                || operands.len() != fields.len()
            {
                return Err(Failure::Shape);
            }
            for ((ordinal, child), value) in fields.iter().zip(operands) {
                path.push(PathPart::Field(*ordinal));
                visit(function, nodes, *child, value, path, result)?;
                path.pop();
            }
        }
        Kind::Enum(variants) => {
            if instruction.kind() != VerifiedInstructionKind::EnumConstruct {
                return Err(Failure::Shape);
            }
            let ordinal = instruction.variant().ok_or(Failure::Shape)?;
            let (_, payload) =
                variants.iter().find(|(variant, _)| *variant == ordinal).ok_or(Failure::Shape)?;
            if operands.len() != usize::from(payload.is_some()) {
                return Err(Failure::Shape);
            }
            if let Some(payload) = payload {
                path.push(PathPart::Variant(ordinal));
                visit(function, nodes, *payload, operands[0], path, result)?;
                path.pop();
            }
        }
        Kind::VecEach { element } | Kind::FixedArray { element, .. } => {
            if let Kind::FixedArray { length, .. } = node.kind() {
                if instruction.kind() != VerifiedInstructionKind::FixedArrayConstruct
                    || u64::try_from(operands.len()).map_err(|_| Failure::Budget)? != *length
                {
                    return Err(Failure::Shape);
                }
            } else {
                if instruction.kind() != VerifiedInstructionKind::VecConstruct {
                    return Err(Failure::Shape);
                }
                result.push(Acquisition {
                    path: path.clone(),
                    operation: LogicalOperation::VecAllocate,
                    source: value,
                });
            }
            for (ordinal, value) in operands.into_iter().enumerate() {
                path.push(PathPart::Index(u32::try_from(ordinal).map_err(|_| Failure::Budget)?));
                visit(function, nodes, *element, value, path, result)?;
                path.pop();
            }
        }
    }
    Ok(())
}

pub(super) fn validate(
    events: &[Acquisition],
    destination: PlaceIdentity,
    survivors: &[PlaceIdentity],
    claim: &Claim,
) -> Result<(), Failure> {
    let event = events.get(claim.ordinal).ok_or(Failure::Ordinal)?;
    if event.operation != claim.operation {
        return Err(Failure::Operation);
    }
    handle_fault_oracle_support::validate_handle_failure(claim.operation, claim.status)
        .map_err(|_| Failure::Status)?;
    if destination != claim.destination {
        return Err(Failure::Destination);
    }
    let expected = events[..claim.ordinal].iter().rev().cloned().collect::<Vec<_>>();
    if expected != claim.reverse_prefix {
        return Err(Failure::Prefix);
    }
    if survivors != claim.survivors {
        return Err(Failure::Survivors);
    }
    Ok(())
}
