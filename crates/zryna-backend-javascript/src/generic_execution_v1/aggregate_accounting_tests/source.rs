//! Existing authenticated helper modules issue genuine same-source NEW seals.
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn with_program(case: &str, provider: &str, check: impl FnOnce(&VerifiedExecution<'_>)) {
    match case {
        "referent-catalogue" => {
            super::catalogue::with_program(case, provider, |program, _| check(program));
        }
        "nominal-shared-clone" | "nested-exclusive-result" | "shared-reborrow-from-exclusive" => {
            crate::generic_execution_v1::borrowed_match_tests::with_program(
                case,
                provider,
                |program, _| check(program),
            );
        }
        "option-nominal-enum-clone" | "fixed-array-identity" => {
            crate::generic_execution_v1::tests::with_program(case, provider, check);
        }
        "partial-root-replacement" | "array-refill" | "failure-after-hole" => {
            super::partial::with_program(case, provider, |program, _| check(program));
        }
        _ => panic!("only unchanged qualified finite fixtures"),
    }
}
#[test]
fn opaque_clone_permission_still_fails_before_any_aggregate_accounting_owner() {
    let expected = super::catalogue::diagnostic_expected();
    let row = expected["cases"]
        .as_array()
        .expect("exact source negatives")
        .iter()
        .find(|row| row["case"] == "opaque-clone")
        .expect("retained opaque Clone hostile");
    for provider in super::catalogue::PROVIDERS {
        let (sources, syntax) = super::catalogue::project("opaque-clone", provider);
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("no opaque permission via closed aggregate effect admission")
        else {
            panic!("source diagnostic");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), row["code"].as_str().expect("frozen code"));
        let span = sources
            .verify_span(zryna_source::UntrustedSpan {
                file: 0,
                start: u32::try_from(row["primary"]["start"].as_u64().expect("start"))
                    .expect("bounded"),
                end: u32::try_from(row["primary"]["end"].as_u64().expect("end")).expect("bounded"),
            })
            .expect("source-owned primary");
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}
