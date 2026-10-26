//! Hit/miss/eviction cost equality, atomic exhaustion and guards before proportional visits.
use super::super::super::plan::MAX_OWNER_UNITS;
use super::super::{Budget, Closed};
use super::{SourceTypeFacts, domain, nominal_key, payloads::Frame};
use crate::generic_v1::{Failure, source::Originals};
mod fixture;
use fixture::Fixture;

fn snapshot(facts: &SourceTypeFacts) -> Option<(Vec<u8>, bool, usize)> {
    facts.last.as_ref().map(|row| (row.key.clone(), row.affine, row.work))
}
fn cached(originals: &Originals<'_>, key: Option<&[u8]>) -> SourceTypeFacts {
    let mut facts = SourceTypeFacts::default();
    if let Some(key) = key {
        facts.nominal(originals, key, &mut Budget::default()).expect("separate cache warmup");
    }
    facts
}
fn budget_failure(result: Result<bool, Failure>) {
    let Failure::Diagnostics(errors) = result.expect_err("exact first excess nominal work rejects")
    else {
        panic!("deterministic budget diagnostic")
    };
    assert_eq!(errors[0].code, "ZRYNA-I3201");
}

#[test]
fn cold_hit_and_evicted_nominal_facts_have_identical_exact_cost_and_first_extra_recovery() {
    let f = Fixture::new();
    let originals = f.originals();
    let marker = nominal_key(0, 1, &[&[2]], &mut Budget::default()).expect("Marker<String>");
    let fixed = nominal_key(0, 2, &[&[1]], &mut Budget::default()).expect("Fixed<i32>");
    let mut measured = Budget::default();
    assert!(
        !SourceTypeFacts::default()
            .nominal(&originals, &marker, &mut measured)
            .expect("all Marker members are Copy despite its String argument")
    );
    let cost = measured.nominal_used();
    for prior in [None, Some(marker.as_slice()), Some(fixed.as_slice())] {
        let mut facts = cached(&originals, prior);
        let mut exact = Budget::original(MAX_OWNER_UNITS - cost);
        assert!(!facts.nominal(&originals, &marker, &mut exact).expect("exact available credit"));
        assert_eq!(exact.nominal_used(), MAX_OWNER_UNITS);
        assert_eq!(facts.last.as_ref().expect("successful cached fact").key, marker);

        let mut facts = cached(&originals, prior);
        let before = snapshot(&facts);
        let start = MAX_OWNER_UNITS - cost + 1;
        let mut short = Budget::original(start);
        budget_failure(facts.nominal(&originals, &marker, &mut short));
        assert_eq!(
            short.nominal_used(),
            start + marker.len() + 1,
            "failed hit and incremental miss publish only the same lookup debit"
        );
        assert_eq!(snapshot(&facts), before, "failed analysis never replaces the last fact");
        let mut retry = Budget::default();
        assert!(!facts.nominal(&originals, &marker, &mut retry).expect("pristine retry"));
        assert_eq!(retry.nominal_used(), cost);
    }
    let mut facts = cached(&originals, Some(&marker));
    let mut measured_fixed = Budget::default();
    assert!(
        facts
            .nominal(&originals, &fixed, &mut measured_fixed)
            .expect("Fixed remains affine even with a Copy argument")
    );
    assert_eq!(facts.last.as_ref().expect("replaced fact").key, fixed);
    let mut retry = Budget::default();
    assert!(
        !facts.nominal(&originals, &marker, &mut retry).expect("evicted original fact recomputes")
    );
    assert_eq!(retry.nominal_used(), cost);
}

#[test]
fn nominal_work_exhausts_before_unknown_header_or_source_node_is_visited() {
    let f = Fixture::new();
    let originals = f.originals();
    let mut unknown = vec![0x11];
    unknown.extend_from_slice(&u32::MAX.to_le_bytes());
    unknown.extend_from_slice(&0u32.to_le_bytes());
    let available = unknown.len() + 1 + unknown.len() * 3 - 1;
    let start = MAX_OWNER_UNITS - available;
    let mut credit = Budget::original(start);
    let mut facts = SourceTypeFacts::default();
    budget_failure(facts.nominal(&originals, &unknown, &mut credit));
    assert_eq!(credit.nominal_used(), start + unknown.len() + 1);
    assert!(facts.last.is_none());
    let Failure::Diagnostics(errors) = facts
        .nominal(&originals, &unknown, &mut Budget::default())
        .expect_err("with work available, an unknown original is still rejected")
    else {
        panic!("identity diagnostic")
    };
    assert_eq!(errors[0].code, "ZRYNA-I7001");

    let declaration = &originals.units[0].data_declarations[1];
    let arguments: [&[u8]; 1] = [&[2]];
    let frame = Frame { originals: &originals, module: 0, declaration, arguments: &arguments };
    let mut exhausted = Budget::original(MAX_OWNER_UNITS);
    budget_failure(frame.shape(u32::MAX, 0, &mut exhausted).map(|shape| shape.affine));
    assert_eq!(exhausted.nominal_used(), MAX_OWNER_UNITS);
}

#[test]
fn substituted_argument_walk_pays_each_node_before_descent_and_legacy_keys_pay_zero() {
    // A root visit was already paid by the original Named<T> occurrence. Only one unit
    // remains here: the first Option child uses it; descent to the malformed grandchild
    // must reject before inspecting that grandchild's tag.
    let malformed = [0x14, 1, 0, 0, 0, 10, 0, 0, 0, 0x14, 1, 0, 0, 0, 1, 0, 0, 0, 0xff];
    let mut credit = Budget::original(MAX_OWNER_UNITS - 1);
    budget_failure(domain::charged_argument(&malformed, &mut credit).map(|shape| shape.affine));
    assert_eq!(credit.nominal_used(), MAX_OWNER_UNITS);

    let f = Fixture::new();
    let originals = f.originals();
    let mut facts = SourceTypeFacts::default();
    let mut legacy = Budget::default();
    for key in [vec![0], vec![1], vec![2], vec![0x14, 1, 0, 0, 0, 1, 0, 0, 0, 1]] {
        let ty = Closed::Stored(key);
        facts.affine(&originals, &ty, &mut legacy).expect("legacy scalar/standard family");
        legacy.nominal_key(&ty, 3).expect("no nominal copy credit on the old lane");
    }
    assert_eq!(legacy.nominal_used(), 0);
    assert!(facts.last.is_none());
}
