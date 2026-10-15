use super::{approval, plan, support};
use zryna_package::SourceTrustMode;
use zryna_source::{SourceFileInput, SourceMap};

#[test]
fn denied_modes_reject_before_invalid_source_plan_and_preserve_complete_prior_state() {
    let mut f = support::Fixture::new();
    let operator = f.approval();
    f.configuration.compiler.sha256 = "not a digest".into();
    let before = f.snapshot();
    for mode in [SourceTrustMode::PureSource, SourceTrustMode::UntrustedPlayground] {
        let mut request = f.request();
        request.step.source.path = "missing.c".into();
        let error = approval::prepare_and_admit(request, mode, operator.clone())
            .expect_err("caller mode wins before malformed plan");
        assert_eq!(error.reason(), "TRUST-OPERATION");
        assert!(!error.detail().is_empty());
        assert_eq!(f.snapshot(), before);
    }
    assert!("trusted-native ".parse::<SourceTrustMode>().is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn source_package_path_and_original_ffi_issuer_substitutions_reject_and_recover() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    for mutation in 0..5 {
        let mut request = f.request();
        match mutation {
            0 => request.step.source.package_id = "0".repeat(64),
            1 => request.step.source.source_sha256 = "0".repeat(64),
            2 => request.step.source.path = "missing.c".into(),
            3 => request.ffi_package_id = "absent",
            _ => request.step.source.path = support::ZRYNA_PATH.into(),
        }
        let error = plan::prepare(request).expect_err("unbound source cannot enter compile plan");
        assert!(matches!(error.reason(), "TRUST-SOURCE" | "TRUST-NATIVE"));
        f.plan();
        assert_eq!(f.snapshot(), before);
    }
    let rebuilt = SourceMap::build(vec![SourceFileInput {
        path: support::ZRYNA_PATH.into(),
        text: support::SOURCE.into(),
    }])
    .expect("equal byte fresh map");
    let mut request = f.request();
    request.sources = &rebuilt;
    assert_eq!(
        plan::prepare(request).expect_err("original issuer cannot be rebuilt from bytes").reason(),
        "TRUST-NATIVE"
    );
    f.plan();
    assert_eq!(f.snapshot(), before);
}

#[test]
fn wrong_tool_target_host_and_independent_operator_bindings_fail_closed() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let mut request = f.request();
    request.step.tool = "undeclared".into();
    assert_eq!(plan::prepare(request).expect_err("undeclared tool").reason(), "TRUST-TOOL");
    for mutation in 0..10 {
        let mut operator = f.approval();
        match mutation {
            0 => operator.source_plan_key = "0".repeat(64),
            1 => operator.recipe_sha256 = "0".repeat(64),
            2 => operator.image_sha256 = "sha256:ambient".into(),
            3 => operator.tool_root_sha256 = "tag".into(),
            4 => operator.policy.configuration_sha256 = "0".repeat(64),
            5 => operator.approved_tools[0].sha256 = "0".repeat(64),
            6 => operator.output_path = "other.o".into(),
            7 => operator.output_bytes = 0,
            8 => operator.output_bytes = zryna_backend_native::MAX_NATIVE_OBJECT_BYTES + 1,
            _ => operator.dependency_symbols = vec!["duplicate".into(), "duplicate".into()],
        }
        assert_eq!(
            approval::prepare_and_admit(f.request(), SourceTrustMode::TrustedNative, operator)
                .expect_err("independent approval must match")
                .reason(),
            "TRUST-NATIVE"
        );
        approval::prepare_and_admit(f.request(), SourceTrustMode::TrustedNative, f.approval())
            .expect("pristine input approval recovers");
        assert_eq!(f.snapshot(), before);
    }
    let mut configuration = f.configuration.clone();
    configuration.host.triple = "aarch64-unknown-linux-gnu".into();
    let mut request = f.request();
    request.configuration = &configuration;
    assert!(plan::prepare(request).is_err());
    let mut configuration = f.configuration.clone();
    configuration.targets[0].triple = "aarch64-unknown-linux-gnu".into();
    let mut request = f.request();
    request.configuration = &configuration;
    assert_eq!(
        plan::prepare(request).expect_err("unsupported native target").reason(),
        "TRUST-NATIVE"
    );
    assert_eq!(f.snapshot(), before);
}
