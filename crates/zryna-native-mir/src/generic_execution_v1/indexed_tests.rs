//! Genuine whole-owner boundaries and private pointer geometry, without target execution.
use super::{LoanAddress, MAX_CODEGEN_UNITS, lower_from};
#[path = "../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
#[test]
fn genuine_indexed_whole_owner_has_exact_extra_overflow_and_pristine_retry() {
    for case in ["array-shared-copy", "chained-shared-copy", "array-indexed-rhs-match"] {
        assert!(!source::text(case).is_empty(), "unchanged authenticated literal");
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _, _| {
                let mut used = 0;
                let first = lower_from(program, &mut used).expect("genuine whole NEW native owner");
                assert!(used > 0);
                let mut cells = 0;
                for function in program.functions() {
                    let plan = &first.functions()[function.index()];
                    for loan in function.loans().filter(|loan| loan.indexed().is_some()) {
                        let Some(LoanAddress::Prepared { slot, offset: 0 }) = plan.loan(loan.id())
                        else {
                            panic!("actual Indexed origin has one prepared address cell")
                        };
                        assert_eq!(slot % 8, 0);
                        assert!(slot.checked_add(8).is_some_and(|end| end <= plan.frame_bytes()));
                        for place in function.places() {
                            let record = plan.place(place.id()).expect("genuine physical place");
                            let size = first.size(record.ty()).expect("same-seal physical size");
                            assert!(
                                u64::from(slot) >= u64::from(record.offset()) + size
                                    || u64::from(slot) + 8 <= u64::from(record.offset()),
                                "pointer cell cannot alias source storage"
                            );
                        }
                        cells += 1;
                    }
                }
                assert!(cells > 0);
                let mut exact = MAX_CODEGEN_UNITS - used;
                lower_from(program, &mut exact).expect("exact whole-owner work");
                assert_eq!(exact, MAX_CODEGEN_UNITS);
                for mut work in [MAX_CODEGEN_UNITS - used + 1, u64::MAX] {
                    let before = work;
                    assert_eq!(
                        lower_from(program, &mut work).err().expect("extra or overflow").code(),
                        "ZRYNA-N7001"
                    );
                    if before == u64::MAX {
                        assert_eq!(work, before, "overflow rejects the first debit atomically");
                    } else {
                        assert!(
                            (before..=MAX_CODEGEN_UNITS).contains(&work),
                            "successful earlier phase debits remain; rejected addition commits no excess"
                        );
                    }
                }
                let mut retry = 0;
                lower_from(program, &mut retry).expect("pristine same-seal retry");
                assert_eq!(retry, used);
            });
        }
    }
}
#[test]
fn opaque_indexed_move_still_fails_at_original_permission_before_native_authority() {
    let case = "opaque-indexed-consume";
    let expected = source::diagnostic(case);
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("source permission precedes native pointer planning")
        else {
            panic!("exact source diagnostic")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), expected["code"].as_str().expect("frozen code"));
    }
}
