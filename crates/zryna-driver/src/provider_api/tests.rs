use super::*;

#[test]
fn native_selection_cannot_expand_source_package_profiles_before_admission() {
    let absent = std::env::temp_dir().join(format!(
        "zryna-native-source-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    assert!(!absent.exists());
    for profile in [CompilationProfile::ControlFlow, CompilationProfile::DataOwnership] {
        let request = ProviderBuildRequest {
            compiler_root: absent.join("compiler"),
            source: CompilationSource::Project(absent.join("project")),
            entrypoint: "src/main.zry".to_owned(),
            artifact_stem: "source-profile".to_owned(),
            targets: TargetSelection::JavaScript,
            profile,
            provider: CompilerProvider::Native,
            node_runtime: None,
        };
        let failure = build_with_provider(&request).expect_err("source package profile is closed");
        assert_eq!(failure.kind(), CommandFailureKind::Request);
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C2001");
        assert!(!absent.exists(), "no source/output admission is attempted");
    }
}
