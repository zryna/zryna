//! Fixed bookkeeping boundaries; these raw map controls grant no source/layout seal.
use super::super::{extension_lookup_tests::with_claim, plan::MAX_OWNER_UNITS};
use super::{ExtensionIndex, preflight, statistics};

#[test]
fn thirteen_units_fit_exactly_and_first_extra_or_overflow_never_reserves_or_commits() {
    with_claim(|_, claim| {
        let mut used = MAX_OWNER_UNITS - 13;
        statistics::reset();
        let index = ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used)
            .expect("fixed F1 B1 D3 E2 debit13 fits exactly");
        assert_eq!(used, MAX_OWNER_UNITS);
        assert_eq!(index.ordinal(0, 1), Some(0));
        assert_eq!(index.ordinal(0, 2), Some(1));
        for result in [0, 3, u32::MAX] {
            assert_eq!(index.ordinal(0, result), None);
        }
        assert_eq!(index.ordinal(usize::MAX, 2), None);
        for initial in [MAX_OWNER_UNITS - 12, MAX_OWNER_UNITS, usize::MAX - 1, usize::MAX] {
            let mut used = initial;
            statistics::reset();
            assert!(ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used).is_err());
            assert_eq!(used, initial, "failed debit cannot change caller credit");
            assert_eq!(statistics::observed(), (Some(initial), None, 0));
        }
    });
}

#[test]
fn late_fill_or_dense_id_failure_rolls_back_and_pristine_replay_adds_only_thirteen() {
    with_claim(|_, original| {
        for attack in 0..5 {
            let mut claim = original.clone();
            match attack {
                0 => claim.extensions[0][1].result = claim.extensions[0][0].result,
                1 => claim.extensions[0][1].result = 3,
                2 => claim.extensions[0][1].result = u32::MAX,
                3 => claim.graph.functions[0].blocks[0].instructions[2].result.id = 99,
                _ => claim.extensions[0].swap(0, 1),
            }
            let mut used = 17;
            statistics::reset();
            assert!(ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used).is_err());
            assert_eq!(used, 17);
            assert_eq!(statistics::observed(), (Some(17), None, 2));
            let index = ExtensionIndex::build(&original.graph, &original.extensions, &mut used)
                .expect("failed partial index cannot contaminate a fresh build");
            assert_eq!(used, 30);
            assert_eq!(index.ordinal(0, 2), Some(1));
        }
    });
}

#[test]
fn empty_outer_and_empty_effect_rows_retain_exact_dimension_debits() {
    with_claim(|_, original| {
        let mut claim = original.clone();
        claim.extensions[0].clear();
        let mut used = 29;
        let index = ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used)
            .expect("raw bookkeeping-only E0 row");
        assert_eq!(used, 40, "2F+3B+2D+E = 2+3+6+0 =11");
        for result in 0..3 {
            assert_eq!(index.ordinal(0, result), None);
        }
        claim.graph.functions.clear();
        claim.extensions.clear();
        let index = ExtensionIndex::build(&claim.graph, &claim.extensions, &mut used)
            .expect("empty outer raw bookkeeping inventory");
        assert_eq!(used, 40);
        assert_eq!(index.ordinal(0, 0), None);
    });
}

#[test]
fn producer_preflight_debits_the_same_counter_without_reserving_a_retained_map() {
    with_claim(|_, claim| {
        statistics::reset();
        let mut used = MAX_OWNER_UNITS - 13;
        preflight(&claim.graph, &claim.extensions, &mut used).expect("exact shared ledger");
        assert_eq!(used, MAX_OWNER_UNITS);
        assert_eq!(statistics::observed(), (None, None, 0));
        let mut used = MAX_OWNER_UNITS - 12;
        assert!(preflight(&claim.graph, &claim.extensions, &mut used).is_err());
        assert_eq!(used, MAX_OWNER_UNITS - 12);
        let mut missing = claim.extensions.clone();
        missing.clear();
        assert!(preflight(&claim.graph, &missing, &mut used).is_err());
        assert_eq!(used, MAX_OWNER_UNITS - 12);
        assert_eq!(statistics::observed(), (None, None, 0));
    });
}

mod legacy;
