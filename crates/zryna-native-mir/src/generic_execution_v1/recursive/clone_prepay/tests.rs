//! Actual source owners bind prepayment to exact finite metadata census and retry.
use super::super::tests::with_program;
use super::{emission, visits};
use crate::generic_execution_v1::{MAX_CODEGEN_UNITS, lower, shapes};
use zryna_layout::StorageTarget;
#[test]
fn genuine_clone_preparation_owns_exact_extra_overflow_and_census_credit() {
    for case in ["drop-only", "clone", "controlled-cleanup", "shared-weak-recursive", "mixed"] {
        with_program(case, |program| {
            let mir = lower(program).expect("actual complete source MIR");
            let plan = mir.containers().expect("actual certified frontiers");
            let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut 0)
                .expect("actual same-owner physical layouts");
            let rows = u64::try_from(types.len()).expect("bounded type count");
            visits::reset();
            let mut cost = 0;
            emission(&types, plan, &mut cost).expect("actual prepaid baseline");
            assert_eq!(visits::count(), rows);
            let mut exact = MAX_CODEGEN_UNITS - cost;
            emission(&types, plan, &mut exact).expect("exact debit");
            assert_eq!(exact, MAX_CODEGEN_UNITS);
            let mut extra = MAX_CODEGEN_UNITS - cost + 1;
            assert_eq!(
                emission(&types, plan, &mut extra).expect_err("first extra").code,
                "ZRYNA-N7001"
            );
            assert!(extra <= MAX_CODEGEN_UNITS);
            let mut overflow = u64::MAX;
            visits::reset();
            assert_eq!(
                emission(&types, plan, &mut overflow).expect_err("overflow").code,
                "ZRYNA-N7001"
            );
            assert_eq!(overflow, u64::MAX);
            assert_eq!(visits::count(), 0);
            let mut short = MAX_CODEGEN_UNITS - rows * 16 + 1;
            visits::reset();
            assert_eq!(
                emission(&types, plan, &mut short).expect_err("census credit").code,
                "ZRYNA-N7001"
            );
            assert_eq!(visits::count(), 0, "no row read before census credit");
            emission(&types, plan, &mut 0).expect("pristine retry");
        });
    }
}
