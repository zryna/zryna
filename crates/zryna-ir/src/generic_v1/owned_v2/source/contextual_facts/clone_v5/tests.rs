//! Authenticated source members drive Clone separately from affinity and argument flags.
use super::super::tests::with;
use super::*;
fn nominal(index: u32, argument: &[u8]) -> Closed {
    let mut key = vec![0x12];
    key.extend(0u32.to_le_bytes());
    key.extend(index.to_le_bytes());
    key.extend(1u32.to_le_bytes());
    key.extend(u32::try_from(argument.len()).expect("small test key").to_le_bytes());
    key.extend(argument);
    Closed::Stored(key)
}
#[test]
fn stored_opaque_is_unclonable_but_phantom_and_fixed_actual_members_provide_clone() {
    for (case, index, expected) in [
        ("opaque-copy-assumption", 0, false),
        ("phantom-remains-copy", 1, true),
        ("fixed-member-remains-affine", 1, true),
    ] {
        with(case, |originals| {
            let frame = Frame { kind: 1, module: 0, index };
            let mut facts = CloneFacts::new(originals, frame, true);
            assert_eq!(
                facts
                    .available(&nominal(index, &[0x30]), &mut Budget::default())
                    .expect("original-frame Clone"),
                expected,
                "{case}"
            );
        });
    }
}
#[test]
fn clone_cold_hit_eviction_and_failed_miss_have_identical_atomic_cost() {
    with("nested-copy", |originals| {
        let frame = Frame { kind: 0, module: 0, index: 0 };
        let ty = nominal(0, &[1]);
        let mut facts = CloneFacts::new(originals, frame, false);
        let mut cold = Budget::default();
        assert!(facts.available(&ty, &mut cold).expect("cold proof"));
        let cost = cold.nominal_used();
        for evict in [false, true] {
            if evict {
                facts.available(&Closed::Stored(vec![2]), &mut Budget::default()).expect("evict");
            }
            let before = facts.last.as_ref().map(|c| (c.key.clone(), c.available, c.work));
            let limit = super::super::super::super::plan::MAX_OWNER_UNITS;
            let mut short = Budget::original(limit - cost + 1);
            assert!(facts.available(&ty, &mut short).is_err());
            let Closed::Stored(key) = &ty else { unreachable!() };
            assert_eq!(short.nominal_used(), limit - cost + 1 + key.len() + 1);
            assert_eq!(facts.last.as_ref().map(|c| (c.key.clone(), c.available, c.work)), before);
            let mut exact = Budget::original(limit - cost);
            assert!(facts.available(&ty, &mut exact).expect("exact retry"));
            assert_eq!(exact.nominal_used(), limit);
        }
    });
}
