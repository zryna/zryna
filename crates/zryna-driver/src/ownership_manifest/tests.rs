use zryna_abi::{ScalarOutcome, ScalarValue};

use super::*;
use crate::{
    DataOwnershipBuildRequest, TargetSelection,
    ownership_pipeline::{
        prepare_data_ownership_for_test,
        test_support::{fixture_workspace, node_executable, route_guard},
    },
};

fn request(root: &std::path::Path, targets: TargetSelection) -> DataOwnershipBuildRequest {
    DataOwnershipBuildRequest {
        workspace_root: root.to_owned(),
        entrypoint: "main.zry".to_owned(),
        artifact_stem: "ownership-score".to_owned(),
        targets,
        node_runtime: node_executable(),
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn canonical_build_manifest_repeats_and_round_trips_exactly() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let success =
        prepare_data_ownership_for_test(&request(workspace.root(), TargetSelection::All), None)
            .expect("candidate build");
    let first =
        render_ownership_manifest_v3(&success, "ownership-score", &[]).expect("manifest v3");
    let second =
        render_ownership_manifest_v3(&success, "ownership-score", &[]).expect("manifest replay");
    assert_eq!(first, second);

    let manifest = decode_ownership_manifest_v3(&first).expect("strict manifest decode");
    assert_eq!(manifest.version(), 3);
    assert_eq!(manifest.profile(), DATA_OWNERSHIP_CANDIDATE_PROFILE);
    assert_eq!(
        manifest.targets(),
        [OwnershipTarget::JavaScript, OwnershipTarget::WebAssembly, OwnershipTarget::Native]
    );
    assert_eq!(manifest.sources.len(), 2);
    assert_eq!(manifest.edges.len(), 1);
}

#[test]
fn run_manifest_requires_ordered_typed_results() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let request = request(workspace.root(), TargetSelection::JavaScript);
    let success = prepare_data_ownership_for_test(
        &request,
        Some(("score".to_owned(), vec![ScalarValue::I32(21)])),
    )
    .expect("candidate run");
    let results = [OwnershipManifestResult::new(
        OwnershipTarget::JavaScript,
        ScalarOutcome::Returned { value: ScalarValue::I32(42) },
    )];
    let bytes = render_ownership_manifest_v3(&success, &request.artifact_stem, &results)
        .expect("run manifest");
    let manifest = decode_ownership_manifest_v3(&bytes).expect("strict run manifest");
    assert_eq!(manifest.results(), results);
    assert!(render_ownership_manifest_v3(&success, &request.artifact_stem, &[]).is_err());
    assert!(render_ownership_manifest_v3(&success, "other-stem", &results).is_err());
}

#[test]
fn hostile_schema_and_identity_changes_are_rejected() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let success = prepare_data_ownership_for_test(
        &request(workspace.root(), TargetSelection::JavaScript),
        None,
    )
    .expect("candidate build");
    let bytes =
        render_ownership_manifest_v3(&success, "ownership-score", &[]).expect("manifest v3");
    let canonical = String::from_utf8(bytes.clone()).expect("UTF-8 manifest");

    let stale = canonical.replace("zryna-data-ownership-v1", "zryna-data-ownership-v1-candidate");
    assert!(decode_ownership_manifest_v3(stale.as_bytes()).is_err());
    let duplicate =
        canonical.replacen("  \"version\": 3,", "  \"version\": 3,\n  \"version\": 3,", 1);
    assert!(decode_ownership_manifest_v3(duplicate.as_bytes()).is_err());
    let unknown = canonical.replacen("\n}\n", ",\n  \"unknown\": true\n}\n", 1);
    assert!(decode_ownership_manifest_v3(unknown.as_bytes()).is_err());
    let missing = canonical.replacen("  \"profile\": \"zryna-data-ownership-v1\",\n", "", 1);
    assert!(decode_ownership_manifest_v3(missing.as_bytes()).is_err());
    let reordered = canonical.replacen(
        "  \"version\": 3,\n  \"profile\": \"zryna-data-ownership-v1\",",
        "  \"profile\": \"zryna-data-ownership-v1\",\n  \"version\": 3,",
        1,
    );
    assert!(decode_ownership_manifest_v3(reordered.as_bytes()).is_err());

    let mut manifest = decode_ownership_manifest_v3(&bytes).expect("manifest");
    let replacement = if manifest.sources[0].sha256.starts_with('0') { "f" } else { "0" };
    manifest.sources[0].sha256.replace_range(0..1, replacement);
    assert!(decode_ownership_manifest_v3(&encode(&manifest).expect("hostile encoding")).is_err());
    let mut manifest = decode_ownership_manifest_v3(&bytes).expect("manifest");
    manifest.artifacts[0].filename = "other.mjs".to_owned();
    assert!(decode_ownership_manifest_v3(&encode(&manifest).expect("hostile encoding")).is_err());
    let mut manifest = decode_ownership_manifest_v3(&bytes).expect("manifest");
    manifest.invocation = Some(ManifestInvocation {
        export: "score".to_owned(),
        arguments: vec![ScalarValue::I32(1)],
    });
    assert!(decode_ownership_manifest_v3(&encode(&manifest).expect("hostile encoding")).is_err());
}

#[test]
fn manifest_byte_budget_accepts_exact_and_rejects_first_extra() {
    assert!(enforce_byte_limit(MAX_OWNERSHIP_MANIFEST_BYTES).is_ok());
    assert!(enforce_byte_limit(MAX_OWNERSHIP_MANIFEST_BYTES + 1).is_err());
}
