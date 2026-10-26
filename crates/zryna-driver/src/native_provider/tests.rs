use super::*;

#[test]
fn native_provider_current_image_enforces_byte_limit_and_exact_protocols() {
    let executable = std::env::current_exe().expect("actual running test image");
    let image_bytes = fs::metadata(&executable).expect("actual running image metadata").len();
    println!("[native-provider-current-image] bytes={image_bytes} ceiling={MAX_COMPILER_BYTES}");
    let captured = NativeProviderCapability::capture_current();
    if image_bytes > MAX_COMPILER_BYTES {
        let Err(error) = captured else {
            panic!(
                "oversized running image must grant no capability: {image_bytes} bytes exceeds {MAX_COMPILER_BYTES}"
            );
        };
        assert_eq!(
            error.code(),
            "ZRYNA-F1101",
            "running image has {image_bytes} bytes; admission ceiling is {MAX_COMPILER_BYTES}"
        );
        return;
    }
    let capability = captured.unwrap_or_else(|error| {
        panic!(
            "retain bounded current image: {image_bytes} bytes; admission ceiling is {MAX_COMPILER_BYTES}: {error:?}"
        )
    });
    assert_eq!(capability.compiler_sha256().len(), 64);
    assert!(capability.compiler_sha256().bytes().all(|byte| byte.is_ascii_hexdigit()));
    capability.revalidate().expect("unchanged current image");
    let v2 = capability.frontend_v2().expect("exact native v2");
    let v3 = capability.frontend_v3().expect("exact native v3");
    let v4 = capability.frontend_v4().expect("exact native v4");
    assert_eq!(v2.spec().expected().provider(), NATIVE_PROVIDER);
    assert_eq!(v3.spec().expected().provider(), NATIVE_PROVIDER);
    assert_eq!(v4.spec().expected().provider(), NATIVE_PROVIDER);
    assert_eq!(v2.spec().expected().provider_version(), NATIVE_PROVIDER_VERSION);
    assert_eq!(v2.spec().arguments(), arguments(2));
    assert_eq!(v3.spec().arguments(), arguments(3));
    assert_eq!(v4.spec().arguments(), arguments(4));
    assert_eq!(v2.spec().limits(), WorkerLimits::default());
    assert_eq!(v3.spec().limits(), WorkerLimitsV3::default());
    assert_eq!(v4.spec().limits(), WorkerLimitsV4::default());
}

#[test]
fn comparison_rejects_two_bootstrap_workers_before_any_source_or_process_effect() {
    use zryna_frontend::VerifiedFrontendProvider;
    let root = std::env::temp_dir();
    let worker = WorkerFrontend::new(
        WorkerSpec::new(
            root.join("never-executed-native-comparison-test"),
            vec![],
            root,
            ProviderExpectation::new(
                "typescript-6",
                "6.0.3",
                2,
                FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
            )
            .expect("exact pinned bootstrap identity"),
            WorkerLimits::default(),
        )
        .expect("absolute never-executed worker specification"),
    );
    let comparison = ComparisonFrontendV2::new(worker.clone(), worker);
    let sources = zryna_source::SourceMap::build(vec![]).expect("bounded empty source map");
    let error = comparison.analyze_verified(&sources).expect_err("wrong configured identities");
    assert_eq!(error.failure(), zryna_frontend::WorkerFailure::SnapshotVerification);
}

#[test]
fn comparison_charges_both_physical_analyses_and_preserves_cleanup_time_reserve() {
    use zryna_frontend::{VerifiedFrontendProviderV3, VerifiedFrontendProviderV4};
    let root = std::env::temp_dir();
    let image = root.join("never-executed-provider-cost-test");
    let make_v3 = |provider| {
        WorkerFrontendV3::new(
            WorkerSpecV3::new(
                &image,
                vec![],
                &root,
                ProviderExpectationV3::new(
                    provider,
                    if provider == NATIVE_PROVIDER { NATIVE_PROVIDER_VERSION } else { "6.0.3" },
                )
                .expect("exact protocol-v3 provider identity"),
                WorkerLimitsV3::default(),
            )
            .expect("absolute bounded protocol-v3 worker specification"),
        )
    };
    let make_v4 = |provider| {
        WorkerFrontendV4::new(
            WorkerSpecV4::new(
                &image,
                vec![],
                &root,
                ProviderExpectationV4::new(
                    provider,
                    if provider == NATIVE_PROVIDER { NATIVE_PROVIDER_VERSION } else { "6.0.3" },
                )
                .expect("exact protocol-v4 provider identity"),
                WorkerLimitsV4::default(),
            )
            .expect("absolute bounded protocol-v4 worker specification"),
        )
    };
    let ordinary_v3 = make_v3(NATIVE_PROVIDER);
    let ordinary_v4 = make_v4(NATIVE_PROVIDER);
    assert_eq!(ordinary_v3.analysis_work_units(), 1);
    assert_eq!(ordinary_v4.analysis_work_units(), 1);
    let compare_v3 = ComparisonFrontendV3::new(ordinary_v3, make_v3("typescript-6"));
    let compare_v4 = ComparisonFrontendV4::new(ordinary_v4, make_v4("typescript-6"));
    assert_eq!(compare_v3.analysis_work_units(), 2);
    assert_eq!(compare_v4.analysis_work_units(), 2);
    assert_eq!(compare_v3.minimum_analysis_timeout(), zryna_frontend::MIN_WORKER_TIMEOUT * 2);
    assert_eq!(compare_v4.minimum_analysis_timeout(), zryna_frontend::MIN_WORKER_TIMEOUT * 2);
}
