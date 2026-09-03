use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::diagnostics::Errors;
use super::owned_control_flow_resources::owned_root_borrow_resource_violation;
use super::type_model::{OwnedRootBorrowSyntax, RootBorrowBudgetLimit};

#[allow(clippy::too_many_lines)]
pub(super) fn postprocess_private_owned_root_borrow_function(
    at: Span,
    plan: &OwnedRootBorrowSyntax,
    mut lowered: raw::Function,
    errors: &mut Errors<'_>,
) -> Option<raw::Function> {
    let root_place =
        lowered.places.iter().find(|place| matches!(place.kind, raw::PlaceKind::Local(0)))?.id;
    let [block] = lowered.blocks.as_slice() else {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one straight-line lowered block",
            "remove control flow and calls from the lexical read-only checkpoint",
        );
        return None;
    };
    let initialize = block.instructions.iter().position(|instruction| {
        matches!(
            instruction.kind,
            raw::InstructionKind::InitializePlace { place, .. } if place == root_place
        )
    })?;
    let consume = block.instructions.iter().rposition(|instruction| {
        matches!(
            instruction.kind,
            raw::InstructionKind::MoveFromPlace { place } if place == root_place
        )
    })?;
    if consume <= initialize {
        return None;
    }
    let [
        raw::SpannedTerminator {
            kind: raw::Terminator::Return { cleanup: return_cleanup, .. },
            ..
        },
    ] = block.terminators.as_slice()
    else {
        errors.at(
            "ZRYNA-M3017",
            at,
            "owned-root borrow reads require one final return",
            "return the original root after the lexical read-only block",
        );
        return None;
    };
    let return_cleanup_index = usize::try_from(return_cleanup.0).ok()?;
    let lexical_owned_drops = lowered
        .cleanup_plans
        .get(return_cleanup_index)?
        .actions
        .iter()
        .map(|action| match action {
            raw::DropAction::DropPlace(place) if *place != root_place => Some(*place),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let existing_transitions = lowered
        .blocks
        .iter()
        .try_fold(0_usize, |count, block| count.checked_add(block.instructions.len()))?;
    if let Some(limit) =
        owned_root_borrow_resource_violation(existing_transitions, lexical_owned_drops.len(), 0)
    {
        let label = match limit {
            RootBorrowBudgetLimit::Transitions => "ownership transitions",
            RootBorrowBudgetLimit::ActiveBorrows => "active borrows",
            _ => unreachable!("owned-root borrow adds only transition and active-borrow costs"),
        };
        errors.at(
            "ZRYNA-M3201",
            at,
            format!("owned-root borrow reads exceed the checked {label} budget"),
            "reduce repeated read-only operations inside the lexical borrow block",
        );
        return None;
    }
    lowered.cleanup_plans.get_mut(return_cleanup_index)?.actions.clear();
    let [block] = lowered.blocks.as_mut_slice() else {
        unreachable!("single-block shape checked before cleanup rewrite")
    };
    let borrow = raw::BorrowId(0);
    block.instructions.insert(
        initialize + 1,
        raw::Instruction {
            result: None,
            span: plan.borrow_at,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: borrow,
                place: root_place,
                access: raw::BorrowAccess::Shared,
                span: plan.borrow_at,
            }),
        },
    );
    let lexical_exit = lexical_owned_drops
        .into_iter()
        .map(|place| raw::Instruction {
            result: None,
            span: plan.end_at,
            kind: raw::InstructionKind::DropPlace { place },
        })
        .chain(std::iter::once(raw::Instruction {
            result: None,
            span: plan.end_at,
            kind: raw::InstructionKind::EndBorrow { borrow },
        }))
        .collect::<Vec<_>>();
    block.instructions.splice((consume + 1)..=consume, lexical_exit);
    Some(lowered)
}
