use super::{Errors, raw, type_at};

pub(super) fn verify_cleanup(
    program: &raw::Program,
    function: &raw::Function,
    errors: &mut Errors,
) {
    if function.cleanup_plans.len() > zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION {
        errors.push("ZRYNA-N3201", "native MIR cleanup plan budget exceeded");
    }
    if function
        .cleanup_plans
        .iter()
        .try_fold(0usize, |count, plan| count.checked_add(plan.actions.len()))
        .is_none_or(|count| count > zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION)
    {
        errors.push("ZRYNA-N3201", "native MIR cleanup action budget exceeded");
        return;
    }
    for (index, plan) in function.cleanup_plans.iter().enumerate() {
        if plan.id as usize != index {
            errors.push("ZRYNA-N3112", "native MIR cleanup identity is not dense");
        }
        for action in &plan.actions {
            let Some(place) = function.places.get(action.place as usize) else {
                errors.push("ZRYNA-N3112", "native MIR cleanup uses an unknown place");
                continue;
            };
            let category = type_at(program, place.ty).map(|ty| ty.category);
            let valid = match action.kind {
                raw::DropKind::Place => {
                    type_at(program, place.ty).is_some_and(|ty| ty.drop_kind != 0)
                }
                raw::DropKind::VecPrefix => category == Some(raw::TypeCategory::Vec),
                raw::DropKind::AggregatePrefix | raw::DropKind::GenericPrefix => matches!(
                    category,
                    Some(
                        raw::TypeCategory::Struct
                            | raw::TypeCategory::Enum
                            | raw::TypeCategory::FixedArray
                            | raw::TypeCategory::Vec
                    )
                ),
            };
            if !valid {
                errors
                    .push("ZRYNA-N3112", "native MIR cleanup kind disagrees with its place layout");
            }
        }
    }
}
