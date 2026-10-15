use super::{approval, plan, support};
use zryna_package::SourceTrustMode;

#[test]
fn retained_resolution_and_original_ffi_issuers_bind_one_compile_only_plan() {
    let f = support::Fixture::new();
    let before = f.snapshot();
    let first = f.plan();
    let other = f.plan();
    assert_eq!(first.source_plan().bytes(), other.source_plan().bytes());
    assert_eq!(first.recipe_sha256(), other.recipe_sha256());
    assert_eq!(first.source_bytes(), support::C_SOURCE);
    assert_eq!(first.sources().identity(), f.sources.identity());
    assert_eq!(first.requirements().object_sha256(), f.requirements.object_sha256());
    assert_eq!(first.recipe_sha256().len(), 64);
    assert_eq!(first.step().sysroots[0].files["library.h"], support::HEADER);
    assert!(support::HEADER.len() > 1024, "header is separately retained sysroot data");
    assert_eq!(first.sysroot_inventory()[0]["files"][0]["sha256"], support::sha(support::HEADER));
    assert_eq!(first.ffi_package_id(), f.resolution.graph().root());
    assert_eq!(first.resolution(), &f.resolution);
    assert_eq!(first.mode(), crate::package_build::PackageBuildMode::Frozen);
    let approved = approval::admit(first, SourceTrustMode::TrustedNative, f.approval())
        .expect("input approval only");
    assert_eq!(approved.mode(), SourceTrustMode::TrustedNative);
    assert_eq!(approved.plan().source_bytes(), support::C_SOURCE);
    assert_eq!(f.snapshot(), before);
}

#[test]
fn replaced_disk_source_and_adjacent_checksum_cannot_replace_retained_compile_bytes() {
    let f = support::Fixture::new();
    let original = f.plan();
    f.rewrite_source(b"malicious replacement C source\n", support::SOURCE.as_bytes());
    let before = f.snapshot();
    assert_eq!(
        f.frozen().expect_err("self-consistent source substitution cannot replace lock").code(),
        "ZRYNA-P4010"
    );
    let retained = f.plan();
    assert_eq!(retained.source_bytes(), support::C_SOURCE);
    assert_eq!(retained.recipe_sha256(), original.recipe_sha256());
    approval::prepare_and_admit(f.request(), SourceTrustMode::TrustedNative, f.approval())
        .expect("retained snapshot remains exact");
    assert_eq!(f.snapshot(), before);
}

#[test]
fn pure_source_bridge_preserves_existing_identity_and_frozen_lock_rejection() {
    let f = support::Fixture::new();
    let bridge = crate::package_build::prepare_source_plan_identity(
        &f.resolution,
        &f.configuration,
        crate::package_build::PackageBuildMode::Frozen,
    )
    .expect("same owner identity");
    assert_eq!(bridge.bytes(), f.plan().source_plan().bytes());
    let update = crate::resolve_package(&crate::PackageResolutionRequest {
        source_root: f.root.clone(),
        package: "pkg".into(),
        git_cache: None,
        mode: crate::PackageLockMode::Update,
    })
    .expect("actual newly published lock");
    let request = f.request_for(&update);
    assert_eq!(
        plan::prepare(request).expect_err("frozen requires pre-existing lock").reason(),
        "ZRYNA-B4101"
    );
    let mut request = f.request_for(&update);
    request.mode = crate::package_build::PackageBuildMode::Offline;
    plan::prepare(request)
        .expect("explicit offline admission can consume already authenticated inputs");
}

#[test]
fn changed_resolved_source_and_new_lock_cannot_reuse_prior_native_recipe_approval() {
    let f = support::Fixture::new();
    let original = f.plan();
    let operator = f.approval();
    let replacement = [support::C_SOURCE, b"\n/* different approved bytes */\n"].concat();
    f.rewrite_source(&replacement, support::SOURCE.as_bytes());
    crate::resolve_package(&crate::PackageResolutionRequest {
        source_root: f.root.clone(),
        package: "pkg".into(),
        git_cache: None,
        mode: crate::PackageLockMode::Update,
    })
    .expect("separate explicit source and lock update");
    let fresh = f.frozen().expect("authenticate the new exact source closure");
    let next = plan::prepare(f.request_for(&fresh)).expect("new snapshot is authentic source data");
    assert_ne!(next.source_plan().cache_key(), original.source_plan().cache_key());
    assert_ne!(next.recipe_sha256(), original.recipe_sha256());
    assert_eq!(next.source_bytes(), replacement);
    let before = f.snapshot();
    assert_eq!(
        approval::admit(next, SourceTrustMode::TrustedNative, operator)
            .expect_err("prior approval cannot authorize new source even after its lock is updated")
            .reason(),
        "TRUST-NATIVE"
    );
    assert_eq!(f.snapshot(), before);
}
