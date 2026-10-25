//! Owning original diagnostics fixed before provider or implementation observations.
use super::source;
use zryna_source::UntrustedSpan;
fn reject(case: &str) {
    let row = source::row(case);
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        let offset = |name: &str| {
            u32::try_from(row["primary"][name].as_u64().expect("frozen UTF8 byte offset"))
                .expect("bounded exact original offset")
        };
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start: offset("start"), end: offset("end") })
            .expect("fixed original source token");
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("owning original rejects invalid source without generated claims")
        else {
            panic!("exact owning source diagnostic, not unfinished/internal rejection");
        };
        assert_eq!(errors.len(), 1, "{case}/{provider}");
        assert_eq!(errors[0].code(), row["code"].as_str().expect("pre-frozen code"));
        assert_eq!(errors[0].primary_span(), Some(span), "{case}/{provider}");
    }
}
#[test]
fn immutable_assignment_rejects_at_the_target_value() {
    reject("immutable-target");
}
#[test]
fn wrong_assignment_type_rejects_at_the_rhs_value() {
    reject("wrong-rhs");
}
#[test]
fn loop_does_not_invent_an_owner_for_the_second_iteration() {
    reject("unrestored-owner");
}
#[test]
fn ordinary_assignment_rechecks_live_shared_aliases() {
    reject("write-live-shared");
}
