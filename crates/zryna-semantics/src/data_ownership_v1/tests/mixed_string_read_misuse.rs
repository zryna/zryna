use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::tests::mixed_string_read_scopes::{ReadCase, read_fixture};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy)]
enum Misuse {
    Type,
    Kind,
    Range,
    OrderedReads,
    ProducedOwner,
    ByteFact,
    MissingRead,
    ExtraRead,
    ConcatOperands,
}

fn change_ranges(plan: &mut PreparationPlan<'_>, at: usize, extra: bool) {
    for step in &mut plan.steps {
        if let Operation::Enter { end, .. } | Operation::StringEnter { end, .. } =
            &mut step.operation
            && *end > at
        {
            *end = if extra { *end + 1 } else { *end - 1 };
        }
    }
}

fn alter(plan: &mut PreparationPlan<'_>, misuse: Misuse) {
    let enter = plan
        .steps
        .iter()
        .position(|step| matches!(step.operation, Operation::StringEnter { .. }))
        .expect("concat scope");
    let read = plan
        .steps
        .iter()
        .position(|step| matches!(step.operation, Operation::StringRead(_)))
        .expect("local read");
    match misuse {
        Misuse::Type => {
            assert_ne!(plan.steps[enter].ty, plan.result_type);
            plan.steps[enter].ty = plan.result_type;
        }
        Misuse::Kind | Misuse::Range | Misuse::OrderedReads => {
            let Operation::StringEnter { kind, end, reads } = &mut plan.steps[enter].operation
            else {
                panic!("concat enter");
            };
            assert_eq!(reads.len(), 2);
            assert_ne!(reads[0].place, reads[1].place);
            match misuse {
                Misuse::Kind => *kind = StringOperation::Clone,
                Misuse::Range => *end += 1,
                Misuse::OrderedReads => reads.swap(0, 1),
                _ => unreachable!(),
            }
        }
        Misuse::ProducedOwner => {
            let step = plan
                .steps
                .iter_mut()
                .find(|step| {
                    matches!(
                        step.operation,
                        Operation::StringRead(StringRead { value: Some(_), .. })
                    )
                })
                .expect("literal produced read");
            let Operation::StringRead(read) = &mut step.operation else { unreachable!() };
            assert_ne!(read.value, Some(raw::ValueId(0)));
            read.value = Some(raw::ValueId(0));
        }
        Misuse::ByteFact => {
            let Operation::StringRead(read) = &mut plan.steps[read].operation else {
                unreachable!()
            };
            assert_eq!(
                read.bytes,
                crate::data_ownership_v1::owned_string_read::StringBytes::Known(1)
            );
            read.bytes = crate::data_ownership_v1::owned_string_read::StringBytes::Known(2);
        }
        Misuse::MissingRead => {
            plan.steps.remove(read);
            change_ranges(plan, read, false);
        }
        Misuse::ExtraRead => {
            let step = &plan.steps[read];
            let Operation::StringRead(value) = step.operation else { unreachable!() };
            assert!(step.owners.is_empty());
            assert!(step.value.is_none());
            let duplicate = super::super::preparation_plan::Step {
                operation: Operation::StringRead(value),
                ty: step.ty,
                at: step.at,
                value: None,
                owners: Vec::new(),
                after: step.after,
            };
            plan.steps.insert(read + 1, duplicate);
            change_ranges(plan, read, true);
        }
        Misuse::ConcatOperands => {
            let step = plan
                .steps
                .iter_mut()
                .find(|step| matches!(step.operation, Operation::Leaf(Leaf::StringConcat { .. })))
                .expect("concat result");
            let Operation::Leaf(Leaf::StringConcat { left, right, .. }) = &mut step.operation
            else {
                unreachable!()
            };
            assert_ne!(left, right);
            std::mem::swap(left, right);
        }
    }
}

fn reject(misuse: Misuse, expected: &str) {
    let (source, snapshot) = read_fixture(ReadCase::LocalConcat);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty));
        let id = root_value(lowerer, 1);
        let mut prepared = PreparedValue::prepare(lowerer, id, ty).expect("valid source prepares");
        alter(&mut prepared.plan, misuse);
        // Internal malformed-plan panics do not promise rollback of materialized state.
        let failure =
            catch_unwind(AssertUnwindSafe(|| prepared.consume())).expect_err("reject misuse");
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .expect("text invariant panic");
        assert!(message.contains(expected), "expected {expected:?}, got {message:?}");
    });
    assert!(errors.is_empty());
}

#[test]
fn mixed_string_scope_rejects_exact_read_and_result_witness_misuse() {
    let (source, snapshot) = read_fixture(ReadCase::LocalConcat);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty));
        let id = root_value(lowerer, 1);
        PreparedValue::prepare(lowerer, id, ty).expect("unmutated control prepares").consume();
    });
    assert!(errors.is_empty());
    for (misuse, expected) in [
        (Misuse::Type, "String read exact type"),
        (Misuse::Kind, "String operation exact arity"),
        (Misuse::Range, "String exit exact range type and parent"),
        (Misuse::OrderedReads, "String read ordered role and identity"),
        (Misuse::ProducedOwner, "String read actual produced owner"),
        (Misuse::ByteFact, "String read actual byte fact"),
        (Misuse::MissingRead, "String read ordered role and identity"),
        (Misuse::ExtraRead, "String read ordered role and identity"),
        (Misuse::ConcatOperands, "String concat ordered read linkage"),
    ] {
        reject(misuse, expected);
    }
}
