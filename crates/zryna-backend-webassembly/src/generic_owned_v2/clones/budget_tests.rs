//! Boundaries exercise the private preflight with an independently issued genuine v5 site.
mod source_fixture;

#[test]
fn genuine_v5_clone_exact_limit_first_extra_overflow_and_pristine_retry() {
    const CAP: usize = 33_554_432;
    assert_eq!(crate::generic_owned_v2::MAX_BYTES, CAP);
    source_fixture::with_nested_prefix(|program| {
        let clone = program.struct_clone(0, 7).expect("frozen genuine CloneStruct result7");
        assert_eq!(clone.source(), 6);
        assert_eq!(clone.result(), 7);
        assert_eq!(clone.string_occurrences(), 3);
        assert_eq!(clone.failure_step().expect("fallible site").cleanup, [6, 0]);

        let mut cost = 0;
        super::preflight(program, clone, &mut cost).expect("actual genuine site fits from zero");
        assert!(cost > 0 && cost < CAP);
        let remaining = CAP.checked_sub(cost).expect("owning work fits real cap");
        let mut exact = remaining;
        super::preflight(program, clone, &mut exact).expect("exact remaining real credit");
        assert_eq!(exact, CAP);

        for prefix in [remaining.checked_add(1).expect("first extra"), usize::MAX] {
            let mut spent = prefix;
            let error = super::preflight(program, clone, &mut spent)
                .expect_err("one less available unit or checked overflow");
            assert_eq!(error.code, "ZRYNA-W4001");
            assert_eq!(spent, prefix, "failed whole-site preflight preserves caller credit");
        }
        let mut retry = 0;
        super::preflight(program, clone, &mut retry)
            .expect("pristine retry with same retained v5 seal");
        assert_eq!(retry, cost, "failed attempts cannot change subsequent site work");
    });
}
