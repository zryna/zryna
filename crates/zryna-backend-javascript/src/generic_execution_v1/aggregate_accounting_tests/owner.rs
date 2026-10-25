//! Actual prepaid eligibility, exact/extra/overflow and foreign-owner rejection.
use super::*;
#[test]
fn qualified_aggregate_sources_activate_the_same_seal_effect_owner() {
    for row in oracle()["full_source"].as_array().expect("eight qualified sources") {
        let case = row["case"].as_str().expect("unchanged source case");
        for provider in catalogue::PROVIDERS {
            source::with_program(case, provider, |program| {
                let context = Context::new(program).expect("genuine source/type/state context");
                let plan = accounting::Plan::derive(&context)
                    .expect("paid actual effect scan")
                    .expect("aggregate paths require real invocation accounting");
                assert!(plan.bound(&context));
            });
        }
    }
}
#[test]
fn aggregate_effect_census_has_exact_first_extra_overflow_and_pristine_retry() {
    source::with_program("referent-catalogue", "native", |program| {
        let context = Context::new(program).expect("genuine complete shape catalogue");
        context.work.set(0);
        let plan = accounting::Plan::derive(&context)
            .expect("paid census")
            .expect("active aggregate owner");
        let cost = context.work.get();
        let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
        assert!(cost > context.types.len());
        context.work.set(limit - cost);
        assert!(accounting::Plan::derive(&context).expect("exact").is_some());
        assert_eq!(context.work.get(), limit);
        context.work.set(limit - cost + 1);
        assert_eq!(
            accounting::Plan::derive(&context).err().expect("first-extra actual census").code(),
            "ZRYNA-J2003"
        );
        assert!(
            (limit - cost + 1..=limit).contains(&context.work.get()),
            "earlier successful debits retained; failed debit commits no excess"
        );
        for before in [limit - context.types.len() + 1, usize::MAX] {
            context.work.set(before);
            assert_eq!(
                accounting::Plan::derive(&context).err().expect("failed first debit").code(),
                "ZRYNA-J2003"
            );
            assert_eq!(context.work.get(), before);
        }
        context.work.set(0);
        assert!(accounting::Plan::derive(&context).expect("pristine").is_some());
        assert_eq!(context.work.get(), cost);
        source::with_program("referent-catalogue", "native", |other| {
            assert!(!plan.bound(&Context::new(other).expect("separately issued identical source")));
        });
    });
}
#[test]
fn pure_copy_array_fixture_stays_zero_allocation_without_dead_accounting_wrappers() {
    for provider in catalogue::PROVIDERS {
        source::with_program("fixed-array-identity", provider, |program| {
            let context = Context::new(program).expect("genuine Copy aggregate source");
            assert!(accounting::Plan::derive(&context).expect("same paid census").is_none());
            let artifact =
                crate::generic_execution_v1::emit(program).expect("old pure Copy ordinary path");
            assert!(!artifact.source.contains("const $accountBrand="));
            assert!(!artifact.source.contains(",accounting:'"));
        });
    }
}
