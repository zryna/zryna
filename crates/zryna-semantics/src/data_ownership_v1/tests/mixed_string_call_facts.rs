use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::super::preparation_plan::CallKind;
use super::*;
use crate::data_ownership_v1::owned_string_read::StringBytes;
use crate::data_ownership_v1::tests::mixed_call_string_nesting::{
    CallStringNesting, nested_call_fixture,
};
use crate::data_ownership_v1::tests::mixed_call_unknown_clone::unknown_call_clone_fixture;
use crate::data_ownership_v1::tests::mixed_string_calls::mixed_string_call_fixture;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use zryna_syntax::v4::RawProjectSyntaxSnapshot;

#[derive(Clone, Copy)]
enum Case {
    Producer,
    Argument,
    Read,
    Clone,
}

fn fixture(case: Case) -> (String, RawProjectSyntaxSnapshot) {
    match case {
        Case::Producer => mixed_string_call_fixture(),
        Case::Argument => nested_call_fixture(CallStringNesting::StringArgument),
        Case::Read => nested_call_fixture(CallStringNesting::StringRead),
        Case::Clone => unknown_call_clone_fixture(),
    }
}

fn final_facts(case: Case) -> BTreeMap<raw::PlaceId, u64> {
    match case {
        Case::Producer | Case::Clone => BTreeMap::from([(raw::PlaceId(1), 1)]),
        Case::Argument => {
            BTreeMap::from([(raw::PlaceId(1), 1), (raw::PlaceId(2), 1), (raw::PlaceId(3), 1)])
        }
        Case::Read => BTreeMap::from([(raw::PlaceId(1), 1), (raw::PlaceId(4), 1)]),
    }
}

fn final_owners(case: Case) -> Vec<raw::PlaceId> {
    let ids = match case {
        Case::Producer => vec![1, 5],
        Case::Argument => vec![1, 2, 3, 7],
        Case::Read => vec![1, 3, 4, 7],
        Case::Clone => vec![1, 3, 6],
    };
    ids.into_iter().map(raw::PlaceId).collect()
}

fn assert_call_facts(plan: &PreparationPlan<'_>, case: Case) {
    let mut entered = Vec::new();
    let mut committed = Vec::new();
    for step in &plan.steps {
        let signature = match &step.operation {
            Operation::CallEnter { signature, .. } => {
                entered.push(signature.id.declaration);
                signature
            }
            Operation::CallCommit { signature, .. } => {
                committed.push((signature.id.declaration, step.value.expect("actual call result")));
                signature
            }
            _ => continue,
        };
        assert_eq!(signature.id.module, raw::ModuleId(0));
        assert_eq!(signature.kind, CallKind::String);
        assert_eq!(signature.result.category, zryna_layout::TypeCategory::String);
        assert_eq!(signature.result, step.ty);
        assert_eq!(
            signature.parameter,
            if signature.id.declaration == 1 { Some(signature.result) } else { None }
        );
        assert_eq!(
            signature.bytes,
            Some(StringBytes::Unknown),
            "never infer Known(4) from the real producer body"
        );
    }
    if matches!(case, Case::Argument) {
        assert_eq!(entered, vec![1]);
        assert_eq!(committed, vec![(1, raw::ValueId(4))]);
    } else {
        assert_eq!(entered, vec![1, 2]);
        assert_eq!(committed, vec![(2, raw::ValueId(1)), (1, raw::ValueId(2))]);
    }
    let clones = plan
        .steps
        .iter()
        .filter_map(|s| match s.operation {
            Operation::Leaf(Leaf::StringClone { bytes, .. }) => Some(bytes),
            _ => None,
        })
        .collect::<Vec<_>>();
    let concats = plan
        .steps
        .iter()
        .filter_map(|s| match s.operation {
            Operation::Leaf(Leaf::StringConcat { bytes, .. }) => Some(bytes),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        clones,
        match case {
            Case::Argument => vec![StringBytes::Known(1)],
            Case::Clone => vec![StringBytes::Unknown],
            _ => vec![],
        }
    );
    assert_eq!(
        concats,
        match case {
            Case::Argument => vec![StringBytes::Known(2)],
            Case::Read => vec![StringBytes::Unknown],
            _ => vec![],
        }
    );
}

fn assert_read_facts(plan: &PreparationPlan<'_>, case: Case) {
    let expected = match case {
        Case::Producer => return,
        Case::Argument => (
            StringOperation::Concat,
            vec![
                (raw::PlaceId(2), StringBytes::Known(1)),
                (raw::PlaceId(3), StringBytes::Known(1)),
            ],
        ),
        Case::Read => (
            StringOperation::Concat,
            vec![(raw::PlaceId(3), StringBytes::Unknown), (raw::PlaceId(4), StringBytes::Known(1))],
        ),
        Case::Clone => (StringOperation::Clone, vec![(raw::PlaceId(3), StringBytes::Unknown)]),
    };
    let scopes = plan
        .steps
        .iter()
        .filter_map(|s| match &s.operation {
            Operation::StringEnter { kind, reads, .. } if *kind == expected.0 => Some(reads),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 1);
    assert_eq!(
        scopes[0].iter().map(|read| (read.place, read.bytes)).collect::<Vec<_>>(),
        expected.1
    );
    assert!(scopes[0].iter().all(|read| read.root == read.place && read.value.is_some()));
}

#[test]
fn mixed_string_calls_keep_opaque_bytes_and_only_surviving_read_local_facts() {
    for case in [Case::Producer, Case::Argument, Case::Read, Case::Clone] {
        let (source, snapshot) = fixture(case);
        for _ in 0..2 {
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
                assert_eq!(
                    lowerer.preparation_facts.string_bytes,
                    BTreeMap::from([(raw::PlaceId(1), 1)])
                );
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let root = root_value(lowerer, 1);
                let prepared = PreparedValue::prepare(lowerer, root, ty)
                    .expect("actual catalog-backed mixed call");
                assert_eq!(state(prepared.lowerer), before);
                assert_eq!(prepared.lowerer.preparation_facts, facts);
                assert_call_facts(&prepared.plan, case);
                assert_read_facts(&prepared.plan, case);
                assert_eq!(prepared.plan.facts.string_bytes, final_facts(case));
                let result = prepared.consume();
                let owners = final_owners(case);
                assert_eq!(lowerer.owners.pending(), owners);
                assert_eq!(lowerer.owners.owner(result), owners.last().copied());
                assert_eq!(
                    lowerer.preparation_facts.string_bytes,
                    final_facts(case),
                    "no facts on consumed argument, opaque result, Struct or Vec"
                );
                assert_eq!(lowerer.preparation_facts.held_cleanup, facts.held_cleanup);
                assert!(lowerer.moved_projections.is_empty());
                assert!(lowerer.partial_roots.is_empty());
            });
            assert!(errors.is_empty());
        }
    }
}

#[test]
fn mixed_string_call_rejects_forged_known_producer_bytes_at_exact_contract_boundary() {
    for matching_entry in [false, true] {
        let (source, snapshot) = mixed_string_call_fixture();
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let root = root_value(lowerer, 1);
            let mut prepared =
                PreparedValue::prepare(lowerer, root, ty).expect("valid opaque producer");
            let mut changed = 0;
            for step in &mut prepared.plan.steps {
                let signature = match &mut step.operation {
                    Operation::CallCommit { signature, .. } if signature.id.declaration == 2 => {
                        signature
                    }
                    Operation::CallEnter { signature, .. }
                        if matching_entry && signature.id.declaration == 2 =>
                    {
                        signature
                    }
                    _ => continue,
                };
                assert_eq!(signature.bytes, Some(StringBytes::Unknown));
                signature.bytes = Some(StringBytes::Known(4));
                changed += 1;
            }
            assert_eq!(changed, if matching_entry { 2 } else { 1 });
            // Deliberately corrupt private plan; no post-panic rollback guarantee is asserted.
            let failure = catch_unwind(AssertUnwindSafe(|| prepared.consume()))
                .expect_err("forged static producer length");
            let text = failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
                .expect("invariant text");
            assert!(
                text.contains(if matching_entry {
                    "call opaque result byte witness"
                } else {
                    "call exact released contract"
                }),
                "{text}"
            );
        });
        assert!(errors.is_empty());
    }
}
