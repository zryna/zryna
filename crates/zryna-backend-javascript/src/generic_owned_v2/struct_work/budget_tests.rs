//! Source-sealed definition-scan accounting at the unchanged JavaScript ceiling.
use super::{source_census::census, source_fixture};
const CAP: usize = 33_554_432;

#[test]
fn actual_wide_sources_seal_before_javascript_definition_scan_accounting() {
    assert_eq!(crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES, CAP);
    for (scopes, expected) in [(1, 18_533), (1024, 5_304_374)] {
        source_fixture::with_wide(scopes, |program| {
            census(program, scopes);
            let mut spent = 0;
            super::precharge_definition_scans(program, &mut spent)
                .expect("unchanged real cap fits");
            assert_eq!(spent, expected, "independently frozen B+I+P+(B+D)*L");
            let artifact = crate::generic_owned_v2::emit(program).expect("actual JS emission");
            assert_eq!(artifact, crate::generic_owned_v2::emit(program).expect("pristine replay"));
        });
    }
}

#[test]
fn definition_scan_exact_first_extra_and_overflow_preserve_caller_credit() {
    source_fixture::with_wide(1, |program| {
        census(program, 1);
        let mut exact = CAP - 18_533;
        super::precharge_definition_scans(program, &mut exact).expect("exact unchanged cap");
        assert_eq!(exact, CAP);
        for prefix in [CAP - 18_532, usize::MAX] {
            let mut spent = prefix;
            let error = super::precharge_definition_scans(program, &mut spent)
                .expect_err("first extra or checked overflow");
            assert_eq!(error.code, "ZRYNA-J2003");
            assert_eq!(spent, prefix, "failed preflight rolls back caller credit");
        }
        let mut retry = 0;
        super::precharge_definition_scans(program, &mut retry).expect("fresh retry");
        assert_eq!(retry, 18_533);
    });
}
