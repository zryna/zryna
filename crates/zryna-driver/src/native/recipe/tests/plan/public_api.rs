use super::support;
use zryna_package::SourceTrustMode;

#[test]
fn public_typed_admission_retains_original_issuers_without_host_effects() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let prepared = crate::prepare_native_recipe(f.request()).expect("public effect-free plan");
    assert_eq!(prepared.source_bytes(), support::C_SOURCE);
    assert_eq!(prepared.resolution(), &f.resolution);
    assert_eq!(prepared.requirements().object_sha256(), f.requirements.object_sha256());
    assert_eq!(prepared.sources().identity(), f.sources.identity());
    assert_eq!(prepared.recipe_sha256(), f.approval().recipe_sha256);
    assert_eq!(prepared.output_path(), "native/adapter.o");
    let error = crate::approve_native_recipe(prepared, SourceTrustMode::PureSource, f.approval())
        .expect_err("public wrapper cannot upgrade a forbidden caller mode");
    assert_eq!(error.reason(), "TRUST-OPERATION");
    assert!(error.to_string().starts_with("TRUST-OPERATION: "));
    let prepared = crate::prepare_native_recipe(f.request()).expect("fresh recovery plan");
    let approved =
        crate::approve_native_recipe(prepared, SourceTrustMode::TrustedNative, f.approval())
            .expect("approval only, not a host operation");
    assert_eq!(approved.mode(), SourceTrustMode::TrustedNative);
    assert_eq!(approved.operator().policy, f.configuration.execution_policy);
    assert_eq!(approved.plan().source_plan().cache_key(), f.plan().source_plan().cache_key());
    assert_eq!(f.snapshot(), before);
}
