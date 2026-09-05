use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::ordinary_array_composition_resources::{LIMITS, counts, parameters, reserve};
use super::*;
use crate::data_ownership_v1::tests::generic_vec_fixture::ordinary_array_composition_fixture::ordinary_array_clone_base_fixture::fixture;
use zryna_ir::data_ownership_v1 as ir;

// One owned array clone, one literal index, one owned element clone, begin/end
// and temporary drop. Two clone cleanup/prefix pairs plus bounds retain roots
// in counts 1+2, 2, and 2+3 respectively. Return cleanup is outside preparation.
const DEMAND: [usize; 5] = [3, 2, 6, 5, 10];

fn seed(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, ty: Ty) -> u32 {
    parameters(lowerer);
    assert!(run_statement(lowerer, 0, ty));
    root_value(lowerer, 1)
}

fn failure_plan(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) {
    let program =
        crate::data_ownership_v1::lower(lowerer.input).expect("authenticated full source");
    let function = program.modules().next().expect("module").functions().next().expect("observe");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let clones = instructions.iter().filter(|i| i.generic_clone().is_some()).collect::<Vec<_>>();
    assert_eq!(clones.len(), 2);
    let base = clones[0].generic_clone().expect("base clone");
    let ir::VerifiedGenericCloneSource::Place(source) = base.source() else {
        panic!("base uses the available source owner");
    };
    assert_ne!(base.destination(), source);
    assert_eq!(
        clones[0].derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![source]
    );
    let prefix = clones[0].generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(prefix.len(), 2);
    assert_eq!(prefix[0].kind(), ir::VerifiedDropActionKind::GenericCloneInitializedPrefix);
    assert_eq!(prefix[0].root(), base.destination());
    assert_eq!(prefix[1].root(), source);
    let bounds = instructions.iter().find(|i| i.indexed_borrow().is_some()).expect("bounds");
    assert_eq!(bounds.indexed_borrow().expect("access").container(), base.destination());
    assert_eq!(
        bounds.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![base.destination(), source]
    );
    let drops = instructions
        .iter()
        .filter(|i| i.kind() == ir::VerifiedInstructionKind::DropPlace)
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].place_operands().collect::<Vec<_>>(), vec![base.destination()]);
}

#[test]
fn ordinary_array_clone_base_resources_exact_first_extra_overflow_and_recovery() {
    // Real authenticated source with synthetic surrounding credits. These are
    // preparation/verified failure-plan checks, not executed allocator faults.
    for dimension in 0..5 {
        let (source, snapshot) = fixture(true, 2, Some(-1), false);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            failure_plan(lowerer);
            let value = seed(lowerer, ty);
            let initial = counts(lowerer);
            let held = LIMITS[dimension] - initial[dimension] - DEMAND[dimension];
            let mut exact_plan = None;
            for credit in [held, held + 1, usize::MAX, held] {
                reserve(lowerer, dimension, credit);
                let before = state(lowerer);
                let checkpoint = lowerer.preparation_checkpoint();
                let facts = lowerer.preparation_facts.clone();
                if credit == held {
                    let prepared = PreparedValue::prepare(lowerer, value, ty)
                        .expect("exact or recovered frontier");
                    assert_eq!(state(prepared.lowerer), before);
                    // Preparation is disposable: repeat the same successful plan
                    // without consuming source or publishing its temporary.
                    drop(prepared);
                    let observed = (state(lowerer), lowerer.preparation_facts.clone());
                    if let Some(previous) = &exact_plan {
                        assert_eq!(&observed, previous);
                    } else {
                        exact_plan = Some(observed);
                    }
                } else {
                    assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
                }
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                assert_eq!(lowerer.preparation_facts, facts);
            }
            PreparedValue::prepare(lowerer, value, ty).expect("recovered consume").consume();
            let actual = counts(lowerer);
            for index in 0..5 {
                assert_eq!(actual[index] - initial[index], DEMAND[index]);
            }
            assert_eq!(actual[dimension] + held, LIMITS[dimension]);
            assert!(lowerer.preparation_facts.active_borrows.is_empty());
            reserve(lowerer, dimension, 0);
            assert!(lowerer.constructor_storage_is_clear());
        });
        assert_eq!(errors.len(), 2);
        for error in errors {
            assert_eq!(error.code(), "ZRYNA-M3201");
        }
    }
}
