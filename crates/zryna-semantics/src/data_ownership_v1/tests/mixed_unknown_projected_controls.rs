use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::owned_string_read::StringBytes;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_unknown_projected::{
    InvalidRead, ProjectedRead, invalid_fixture, projected_fixture,
};
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawExpressionKind;

fn assert_read_facts(plan: &PreparationPlan<'_>, cloned: bool) {
    let reads =
        plan.steps
            .iter()
            .filter_map(|s| {
                if let Operation::StringRead(read) = &s.operation { Some(read) } else { None }
            })
            .collect::<Vec<_>>();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0].bytes, StringBytes::Unknown);
    assert_eq!(reads[0].place, raw::PlaceId(if cloned { 4 } else { 3 }));
    assert_eq!(reads[0].root, raw::PlaceId(if cloned { 4 } else { 2 }));
    assert_eq!(reads[0].value, if cloned { Some(raw::ValueId(3)) } else { None });
    assert_eq!(reads[1].bytes, StringBytes::Known(1));
    let concats = plan
        .steps
        .iter()
        .filter_map(|s| {
            if let Operation::Leaf(Leaf::StringConcat { bytes, .. }) = &s.operation {
                Some(*bytes)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(concats, vec![StringBytes::Unknown]);
    if cloned {
        let clone_facts = plan
            .steps
            .iter()
            .filter_map(|s| {
                if let Operation::Leaf(Leaf::StringClone { bytes, .. }) = &s.operation {
                    Some(*bytes)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(clone_facts, vec![StringBytes::Unknown]);
    }
}

#[test]
fn mixed_unknown_projected_reads_bind_real_availability_and_cached_prefix_facts() {
    for mode in [ProjectedRead::Direct, ProjectedRead::Clone] {
        let (source, snapshot) = projected_fixture(mode);
        let cloned = matches!(mode, ProjectedRead::Clone);
        for cached in [false, true] {
            for _ in 0..2 {
                let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                    assert!(run_statement(lowerer, 0, ty));
                    assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(2)]);
                    assert!(
                        lowerer.preparation_facts.string_bytes.is_empty(),
                        "real String fact was consumed into the initialized Pair"
                    );
                    let projection_id = lowerer
                        .function
                        .body
                        .expressions
                        .iter()
                        .position(|e| matches!(e.kind, RawExpressionKind::FieldAccess { .. }))
                        .and_then(|id| u32::try_from(id).ok())
                        .expect("actual p.first");
                    let cached_span = if cached {
                        let source =
                            lowerer.owned_place(projection_id).expect("real cached prefix");
                        assert_eq!(source.place, raw::PlaceId(3));
                        assert!(
                            !lowerer.preparation_facts.string_bytes.contains_key(&source.place)
                        );
                        Some(lowerer.places[3].span)
                    } else {
                        None
                    };
                    let before = state(lowerer);
                    let facts = lowerer.preparation_facts.clone();
                    let root = root_value(lowerer, 1);
                    let prepared =
                        PreparedValue::prepare(lowerer, root, ty).expect("available Unknown read");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_facts, facts);
                    assert_eq!(
                        prepared
                            .plan
                            .steps
                            .iter()
                            .filter(|s| matches!(s.operation, Operation::Prefix { .. }))
                            .count(),
                        usize::from(!cached)
                    );
                    assert_read_facts(&prepared.plan, cloned);
                    let value = prepared.consume();
                    assert_eq!(lowerer.projections.get(&(2, 0, 0)), Some(&raw::PlaceId(3)));
                    if let Some(at) = cached_span {
                        assert_eq!(lowerer.places[3].span, at);
                    }
                    assert_eq!(
                        lowerer.preparation_facts.string_bytes,
                        BTreeMap::from([(raw::PlaceId(if cloned { 5 } else { 4 }), 1)]),
                        "only right literal has a known fact; Unknown is never known zero"
                    );
                    assert_eq!(
                        lowerer.owners.pending(),
                        if cloned {
                            vec![raw::PlaceId(2), raw::PlaceId(4), raw::PlaceId(5), raw::PlaceId(8)]
                        } else {
                            vec![raw::PlaceId(2), raw::PlaceId(4), raw::PlaceId(7)]
                        }
                    );
                    assert_eq!(
                        lowerer.owners.owner(value),
                        Some(raw::PlaceId(if cloned { 8 } else { 7 }))
                    );
                    assert!(lowerer.moved_projections.is_empty());
                    assert!(lowerer.partial_roots.is_empty());
                    assert!(lowerer.errors.is_empty());
                });
                assert!(errors.is_empty());
            }
        }
    }
}

#[test]
fn mixed_unknown_projected_fact_never_rescues_invalid_type_or_unavailable_owner() {
    for mode in [
        InvalidRead::Missing,
        InvalidRead::WrongType,
        InvalidRead::MovedRoot,
        InvalidRead::MovedLeaf,
    ] {
        let (source, snapshot) = invalid_fixture(mode);
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                let moved = matches!(mode, InvalidRead::MovedRoot | InvalidRead::MovedLeaf);
                if moved {
                    assert!(run_statement(lowerer, 1, ty), "real preceding move statement");
                }
                let bad = if matches!(mode, InvalidRead::Missing) {
                    lowerer.function.body.expressions.iter().find(|e|
                        matches!(&e.kind, RawExpressionKind::Reference { name } if name.text == "q"))
                        .expect("missing source name").span
                } else {
                    lowerer
                        .function
                        .body
                        .expressions
                        .iter()
                        .rev()
                        .find(|e| matches!(e.kind, RawExpressionKind::FieldAccess { .. }))
                        .expect("final source projection")
                        .span
                };
                let (code, message, help) = match mode {
                    InvalidRead::Missing => (
                        "ZRYNA-M3002",
                        "aggregate value 'q' is not declared",
                        "reference one exact preceding local using its declared spelling",
                    ),
                    InvalidRead::WrongType => (
                        "ZRYNA-M3012",
                        "projected String clone requires one exact static String leaf",
                        "clone an initialized Struct field or constant fixed-array String element",
                    ),
                    InvalidRead::MovedRoot | InvalidRead::MovedLeaf => (
                        "ZRYNA-M3014",
                        "projected String clone source is moved or overlaps a moved subobject",
                        "clone only an initialized available static String projection",
                    ),
                };
                expected = Some(Diagnostic::error_at(
                    code,
                    span(lowerer.input.sources(), bad),
                    message,
                    help,
                ));
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let root = root_value(lowerer, if moved { 2 } else { 1 });
                assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                assert_eq!(
                    state(lowerer),
                    before,
                    "prior real statements and complete state retained"
                );
                assert_eq!(lowerer.preparation_facts, facts);
            });
            assert_eq!(errors, [expected.expect("exact authenticated diagnostic")]);
        }
    }
}
