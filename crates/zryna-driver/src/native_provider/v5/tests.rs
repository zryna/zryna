use super::*;
#[test]
fn retained_current_native_image_builds_exact_v5_without_selecting_a_profile() {
    let image = std::env::current_exe().expect("actual image");
    let size = std::fs::metadata(image).expect("image metadata").len();
    let captured = NativeProviderCapability::capture_current();
    if size > super::super::MAX_COMPILER_BYTES {
        assert_eq!(captured.expect_err("existing image ceiling").code(), "ZRYNA-F1101");
        return;
    }
    let capability = captured.expect("bounded retained compiler");
    let worker = capability.frontend_v5().expect("typed V5 configuration");
    assert_eq!(worker.spec().arguments(), arguments(5));
    assert_eq!(worker.spec().expected().provider(), NATIVE_PROVIDER);
    assert_eq!(worker.spec().expected().provider_version(), NATIVE_PROVIDER_VERSION);
    let caps = worker.spec().expected().capabilities();
    assert!(!caps.module_resolution && !caps.semantic_diagnostics);
    assert!(
        caps.control_flow_v1 && caps.data_ownership_syntax_v1 && caps.bounded_generics_syntax_v1
    );
    assert_eq!(worker.spec().limits(), WorkerLimitsV5::default());
    capability.revalidate().expect("constructor preserves current-image authority");
}
