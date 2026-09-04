use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::owned_string_read::StringBytes;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_disjoint_owned_sibling::sibling_fixture;
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawExpressionKind;

#[test]
fn mixed_disjoint_owned_sibling_preparation_retains_prior_owner_masks_and_facts() {
    for disjoint in [true, false] {
        let (source, snapshot) = sibling_fixture(disjoint);
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                assert!(run_statement(lowerer, 1, ty));
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(3), raw::PlaceId(6)]);
                assert_eq!(lowerer.moved_projections, BTreeSet::from([raw::PlaceId(4)]));
                assert_eq!(lowerer.partial_roots, BTreeSet::from([raw::PlaceId(3)]));
                assert_eq!(lowerer.places.len(), 7);
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let bad = lowerer
                    .function
                    .body
                    .expressions
                    .iter()
                    .rev()
                    .find(|e| matches!(e.kind, RawExpressionKind::FieldAccess { .. }))
                    .expect("later p.first")
                    .span;
                let root = root_value(lowerer, 2);
                if !disjoint {
                    expected = Some(Diagnostic::error_at(
                        "ZRYNA-M3014",
                        span(lowerer.input.sources(), bad),
                        "projected String clone source is moved or overlaps a moved subobject",
                        "clone only an initialized available static String projection",
                    ));
                    assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_facts, facts);
                    return;
                }
                let prepared = PreparedValue::prepare(lowerer, root, ty)
                    .expect("disjoint owned sibling remains available");
                assert_eq!(state(prepared.lowerer), before);
                assert_eq!(prepared.lowerer.preparation_facts, facts);
                let reads = prepared
                    .plan
                    .steps
                    .iter()
                    .filter_map(|step| {
                        if let Operation::StringRead(read) = &step.operation {
                            Some(*read)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(reads.len(), 2);
                assert_eq!(
                    (reads[0].place, reads[0].root, reads[0].value, reads[0].bytes),
                    (raw::PlaceId(7), raw::PlaceId(3), None, StringBytes::Unknown)
                );
                assert_eq!(
                    (reads[1].place, reads[1].value, reads[1].bytes),
                    (raw::PlaceId(8), Some(raw::ValueId(4)), StringBytes::Known(1))
                );
                let value = prepared.consume();
                assert_eq!(value, raw::ValueId(8));
                assert_eq!((lowerer.instructions.len(), lowerer.places.len()), (11, 13));
                assert_eq!(
                    lowerer.owners.pending(),
                    &[raw::PlaceId(3), raw::PlaceId(6), raw::PlaceId(8), raw::PlaceId(12),]
                );
                assert_eq!(lowerer.owners.owner(value), Some(raw::PlaceId(12)));
                assert_eq!(lowerer.projections.get(&(3, 0, 1)), Some(&raw::PlaceId(4)));
                assert_eq!(lowerer.projections.get(&(3, 0, 0)), Some(&raw::PlaceId(7)));
                assert_eq!(lowerer.moved_projections, BTreeSet::from([raw::PlaceId(4)]));
                assert_eq!(lowerer.partial_roots, BTreeSet::from([raw::PlaceId(3)]));
                assert_eq!(
                    lowerer.preparation_facts.string_bytes,
                    BTreeMap::from([(raw::PlaceId(8), 1)])
                );
            });
            if disjoint {
                assert!(errors.is_empty());
            } else {
                assert_eq!(errors, [expected.expect("exact source diagnostic")]);
            }
        }
    }
}
