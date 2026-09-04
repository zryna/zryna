use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_vec_siblings::vec_sibling_fixture;
use zryna_diagnostics::Diagnostic;

#[test]
fn mixed_vec_call_sibling_preparation_preserves_real_catalog_local_state_and_facts() {
    for duplicate in [false, true] {
        let (source, snapshot) = vec_sibling_fixture(duplicate);
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                // The harness builds the actual function catalog from authenticated source.
                assert!(run_statement(lowerer, 0, ty), "real preceding Vec producer local");
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
                assert!(lowerer.preparation_facts.string_bytes.is_empty());
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let root = root_value(lowerer, 1);
                if duplicate {
                    let bad = lowerer.function.body.expressions[3].span;
                    expected = Some(Diagnostic::error_at(
                        "ZRYNA-M3014",
                        span(lowerer.input.sources(), bad),
                        "aggregate value 'items' is moved or only partially available",
                        "move a whole owned aggregate only before moving any of its projections",
                    ));
                    assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                    assert_eq!(
                        state(lowerer),
                        before,
                        "all real state including constructor cache retained"
                    );
                    assert_eq!(lowerer.preparation_facts, facts);
                } else {
                    let prepared = PreparedValue::prepare(lowerer, root, ty)
                        .expect("independent Vec sibling source");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_facts, facts);
                    let value = prepared.consume();
                    assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(5)]);
                    assert_eq!(lowerer.owners.owner(value), Some(raw::PlaceId(5)));
                    assert!(
                        lowerer.preparation_facts.string_bytes.is_empty(),
                        "Vec/Struct call transfer never creates String facts"
                    );
                    assert_eq!(lowerer.preparation_facts.held_cleanup, facts.held_cleanup);
                }
            });
            assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
        }
    }
}
