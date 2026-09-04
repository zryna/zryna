use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::tests::mixed_string_read_scopes::{ReadCase, read_fixture};
use std::collections::BTreeMap;

#[test]
fn mixed_string_read_scopes_preserve_exact_live_byte_facts_and_owner_order() {
    for case in [ReadCase::LiteralClone, ReadCase::LocalConcat, ReadCase::NestedConcat] {
        let (source, snapshot) = read_fixture(case);
        let nested = matches!(case, ReadCase::NestedConcat);
        for _ in 0..2 {
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
                assert_eq!(
                    lowerer.preparation_facts.string_bytes,
                    BTreeMap::from([(raw::PlaceId(1), 1)])
                );
                let before = state(lowerer);
                let before_facts = lowerer.preparation_facts.clone();
                let id = root_value(lowerer, 1);
                let prepared = PreparedValue::prepare(lowerer, id, ty)
                    .expect("authenticated mixed String read preparation");
                assert_eq!(state(prepared.lowerer), before, "preparation does not materialize");
                assert_eq!(prepared.lowerer.preparation_facts, before_facts);
                let expected_facts = if nested {
                    BTreeMap::from([
                        (raw::PlaceId(1), 1),
                        (raw::PlaceId(2), 1),
                        (raw::PlaceId(3), 1),
                    ])
                } else {
                    BTreeMap::from([(raw::PlaceId(1), 1), (raw::PlaceId(2), 1)])
                };
                assert_eq!(prepared.plan.facts.string_bytes, expected_facts);
                let expected_pending = if nested {
                    vec![raw::PlaceId(1), raw::PlaceId(2), raw::PlaceId(3), raw::PlaceId(6)]
                } else {
                    vec![raw::PlaceId(1), raw::PlaceId(2), raw::PlaceId(5)]
                };
                assert_eq!(prepared.plan.owners.pending(), expected_pending);
                let concats = prepared
                    .plan
                    .steps
                    .iter()
                    .filter_map(|step| {
                        if let Operation::Leaf(Leaf::StringConcat { bytes, .. }) = &step.operation {
                            bytes.known()
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    concats,
                    if matches!(case, ReadCase::LiteralClone) { vec![] } else { vec![2] }
                );
                let value = prepared.consume();
                assert_eq!(lowerer.owners.pending(), expected_pending);
                assert_eq!(lowerer.preparation_facts.string_bytes, expected_facts);
                assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
                let returned = lowerer.owners.owner(value).expect("final Vec owner");
                assert_eq!(returned, raw::PlaceId(if nested { 6 } else { 5 }));
                assert!(
                    !lowerer.preparation_facts.string_bytes.contains_key(&returned),
                    "Vec cannot inherit the consumed String result's byte fact"
                );
                assert_eq!(
                    lowerer.bindings.get("text").expect("retained local").place,
                    raw::PlaceId(1)
                );
                assert!(lowerer.moved_projections.is_empty());
                assert!(lowerer.partial_roots.is_empty());
                assert!(lowerer.errors.is_empty());
            });
            assert!(errors.is_empty());
        }
    }
}
