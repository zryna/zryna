use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::super::preparation_plan::{CallKind, PreparationPlan};
use super::*;
use crate::data_ownership_v1::tests::mixed_string_calls::mixed_string_call_fixture;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug)]
enum Damage {
    ForeignModule,
    WrongKind,
    WrongCallee,
    WrongVecElement,
    EntryArity,
    NestedRange,
    TransferValue,
    TransferOwner,
    MissingTransfer,
    CommitArguments,
    CommitCallee,
    Cleanup,
    Result,
}

fn damage_entry(
    plan: &mut PreparationPlan<'_>,
    root_ty: Ty,
    damage: Damage,
) -> Option<&'static str> {
    let parent_end = plan
        .steps
        .iter()
        .find_map(|step| match step.operation {
            Operation::CallEnter { signature, end, .. } if signature.parameter.is_some() => {
                Some(end)
            }
            _ => None,
        })
        .expect("identity parent range");
    let step = plan.steps.iter_mut().find(|step| matches!(
        step.operation, Operation::CallEnter { signature, .. } if signature.parameter.is_none()
    )).expect("actual producer entry");
    let Operation::CallEnter { signature, arguments, end } = &mut step.operation else {
        unreachable!()
    };
    assert_eq!(signature.id.declaration, 2);
    match damage {
        Damage::ForeignModule => {
            signature.id.module = raw::ModuleId(1);
            Some("call same module authority")
        }
        Damage::WrongKind => {
            signature.kind = CallKind::Vec;
            Some("call category linkage")
        }
        Damage::WrongCallee => {
            signature.id.declaration = 1;
            Some("call actual parameter signature")
        }
        Damage::WrongVecElement => {
            // The real make declaration returns Vec<Parcel>, not an admitted Vec<String>.
            assert_eq!(root_ty.category, zryna_layout::TypeCategory::Vec);
            signature.id.declaration = 0;
            signature.result = root_ty;
            signature.kind = CallKind::Vec;
            signature.bytes = None;
            step.ty = root_ty;
            Some("call exact Vec String authority")
        }
        Damage::NestedRange => {
            assert!(*end < parent_end);
            *end = parent_end;
            Some("nested call range")
        }
        Damage::EntryArity => {
            assert!(arguments.is_empty());
            arguments.push(raw::ValueId(0));
            Some("call exact argument arity")
        }
        _ => None,
    }
}

fn damage_plan(plan: &mut PreparationPlan<'_>, root_ty: Ty, damage: Damage) -> &'static str {
    if let Some(message) = damage_entry(plan, root_ty, damage) {
        return message;
    }
    if matches!(damage, Damage::MissingTransfer) {
        let index = plan
            .steps
            .iter()
            .position(|step| matches!(step.operation, Operation::CallTransfer { .. }))
            .expect("identity transfer");
        plan.steps.remove(index);
        for step in &mut plan.steps {
            if let Operation::Enter { end, .. }
            | Operation::StringEnter { end, .. }
            | Operation::CallEnter { end, .. } = &mut step.operation
                && *end > index
            {
                *end -= 1;
            }
        }
        return "call complete argument transfer";
    }
    if matches!(damage, Damage::TransferValue | Damage::TransferOwner) {
        let step = plan
            .steps
            .iter_mut()
            .find(|step| matches!(step.operation, Operation::CallTransfer { .. }))
            .expect("identity transfer");
        let Operation::CallTransfer { value, owner } = &mut step.operation else { unreachable!() };
        if matches!(damage, Damage::TransferValue) {
            assert_ne!(*value, raw::ValueId(0));
            *value = raw::ValueId(0);
            return "call ordered transfer value";
        }
        assert_ne!(*owner, raw::PlaceId(1));
        *owner = raw::PlaceId(1);
        return "call actual argument owner";
    }
    let step = plan
        .steps
        .iter_mut()
        .find(|step| {
            matches!(step.operation,
                Operation::CallCommit { signature, .. } if signature.parameter.is_some()
            )
        })
        .expect("identity commit");
    let Operation::CallCommit { signature, arguments, cleanup } = &mut step.operation else {
        unreachable!()
    };
    match damage {
        Damage::CommitArguments => {
            assert_eq!(arguments.len(), 1);
            assert_ne!(arguments[0], raw::ValueId(0));
            arguments[0] = raw::ValueId(0);
            "call committed ordered operands"
        }
        Damage::CommitCallee => {
            signature.id.declaration = 2;
            "call exact released contract"
        }
        Damage::Cleanup => {
            assert_ne!(*cleanup, raw::CleanupPlanId(0));
            *cleanup = raw::CleanupPlanId(0);
            "call cleanup linkage"
        }
        Damage::Result => {
            assert_ne!(step.value, Some(raw::ValueId(0)));
            step.value = Some(raw::ValueId(0));
            "prepared value identity"
        }
        _ => unreachable!("entry and transfer cases already handled"),
    }
}

#[test]
fn mixed_call_consumption_rejects_precise_catalog_transfer_cleanup_and_result_corruption() {
    let cases = [
        Damage::ForeignModule,
        Damage::WrongKind,
        Damage::WrongCallee,
        Damage::WrongVecElement,
        Damage::EntryArity,
        Damage::NestedRange,
        Damage::TransferValue,
        Damage::TransferOwner,
        Damage::MissingTransfer,
        Damage::CommitArguments,
        Damage::CommitCallee,
        Damage::Cleanup,
        Damage::Result,
    ];
    for damage in std::iter::once(None).chain(cases.into_iter().map(Some)) {
        let (source, snapshot) = mixed_string_call_fixture();
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty), "authentic preceding local");
            let root = root_value(lowerer, 1);
            let mut prepared = PreparedValue::prepare(lowerer, root, ty).expect("valid mixed call");
            let Some(damage) = damage else {
                prepared.consume();
                return;
            };
            let expected = damage_plan(&mut prepared.plan, ty, damage);
            // Internal malformed-plan rejection is not a promise of rollback after panic.
            let failure = catch_unwind(AssertUnwindSafe(|| prepared.consume()))
                .expect_err("corrupt call contract");
            let text = failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
                .expect("invariant text");
            assert!(text.contains(expected), "{damage:?}: expected {expected}, got {text}");
        });
        assert!(errors.is_empty());
    }
}
