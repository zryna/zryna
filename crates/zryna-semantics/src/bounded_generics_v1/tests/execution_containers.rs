//! Frozen container sources reach the WHOLE new issuer, not only isolated phase helpers.
use zryna_ir::generic_v1::execution_v1 as execution;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
mod authority;
mod fixture;
mod views;
#[test]
fn nine_frozen_literals_authenticate_both_complete_provider_packets() {
    let expected = fixture::expected();
    let cases = expected["cases"].as_array().expect("nine original cases");
    assert_eq!(cases.len(), 9);
    for row in cases {
        let case = row["case"].as_str().expect("original case identity");
        let (left_sources, left) = fixture::project(case, "native");
        let (right_sources, right) = fixture::project(case, "typescript");
        assert!(left.is_bound_to(&left_sources));
        assert!(right.is_bound_to(&right_sources));
        assert_eq!(left.files(), right.files(), "{case}: full DTO equality");
        assert_ne!(left_sources.identity(), right_sources.identity());
    }
}
fn assert_seals(case: &str) {
    let mut count = 0;
    for provider in fixture::PROVIDERS {
        fixture::with_claim(case, provider, |sources, syntax, linear, linux, claim| {
            let runtime = runtime::verify_v1(
                runtime::raw_v1(linear, linux).expect("genuine runtime declarations"),
                linear,
                linux,
            )
            .expect("independent runtime authority");
            let bytes = execution::wire::encode(claim).expect("whole bounded wire");
            assert!(bytes.starts_with(execution::wire::HEADER));
            assert!(zryna_ir::generic_v1::owned_v2::wire::v5::decode(&bytes).is_err());
            let program = execution::verify(
                execution::wire::decode(&bytes).expect("new domain carrier"),
                syntax,
                sources,
                sources.verify_file_id(0).expect("original entry"),
                linear,
                linux,
                &runtime,
            )
            .expect("WHOLE independent source/type/state/plan/ABI issuer");
            assert_eq!(program.entry_file(), sources.verify_file_id(0).expect("entry"));
            assert!(std::ptr::eq(program.layouts(zryna_layout::StorageTarget::Linear32V1), linear));
            assert!(std::ptr::eq(
                program.layouts(zryna_layout::StorageTarget::LinuxX8664V1),
                linux
            ));
            assert!(std::ptr::eq(program.runtime(), &raw const runtime));
            assert_eq!(program.scalar_abi().exports().len(), 1);
            assert_eq!(
                program
                    .scalar_abi()
                    .exports()
                    .next()
                    .expect("entry export")
                    .logical_name()
                    .as_str(),
                "entry"
            );
            assert_eq!(program.export_functions().len(), 1);
            views::check(case, &program);
            count += 1;
        });
    }
    assert_eq!(count, 2, "both honest provider source brands");
}

#[test]
fn vec_copy_genuine_source_issues_the_whole_seal() {
    assert_seals("vec-copy");
}
#[test]
fn vec_owned_preparation_genuine_source_issues_the_whole_seal() {
    assert_seals("vec-owned-prepare");
}
#[test]
fn shared_and_weak_count_genuine_source_issues_the_whole_seal() {
    assert_seals("shared-weak-count");
}
#[test]
fn unused_opaque_count_templates_genuine_source_issue_the_whole_seal() {
    assert_seals("opaque-count-unused");
}
