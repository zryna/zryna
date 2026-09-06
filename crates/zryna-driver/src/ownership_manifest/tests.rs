use std::{env, path::PathBuf};

use zryna_abi::{ScalarOutcome, ScalarValue};

use super::*;
use crate::{
    DataOwnershipBuildRequest, TargetSelection,
    ownership_pipeline::{OWNERSHIP_ROUTE_TEST_LOCK, prepare_data_ownership_for_test},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repository root")
}

fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .chain(
            env::var_os("PATH")
                .into_iter()
                .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
                .map(move |directory| directory.join(executable)),
        )
        .find(|path| path.is_file())
        .expect("Node.js")
        .canonicalize()
        .expect("canonical Node.js")
}

fn request(targets: TargetSelection) -> DataOwnershipBuildRequest {
    DataOwnershipBuildRequest {
        workspace_root: root(),
        entrypoint: "tests/m3-fixtures/candidate-modules/main.zry".to_owned(),
        artifact_stem: "ownership-score".to_owned(),
        targets,
        node_runtime: node_executable(),
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn canonical_build_manifest_repeats_and_round_trips_exactly() {
    let _guard = OWNERSHIP_ROUTE_TEST_LOCK.lock().expect("route test lock");
    let success = prepare_data_ownership_for_test(&request(TargetSelection::All), None)
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
    let _guard = OWNERSHIP_ROUTE_TEST_LOCK.lock().expect("route test lock");
    let request = request(TargetSelection::JavaScript);
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
    let _guard = OWNERSHIP_ROUTE_TEST_LOCK.lock().expect("route test lock");
    let success = prepare_data_ownership_for_test(&request(TargetSelection::JavaScript), None)
        .expect("candidate build");
    let bytes =
        render_ownership_manifest_v3(&success, "ownership-score", &[]).expect("manifest v3");
    let canonical = String::from_utf8(bytes.clone()).expect("UTF-8 manifest");

    let duplicate =
        canonical.replacen("  \"version\": 3,", "  \"version\": 3,\n  \"version\": 3,", 1);
    assert!(decode_ownership_manifest_v3(duplicate.as_bytes()).is_err());
    let unknown = canonical.replacen("\n}\n", ",\n  \"unknown\": true\n}\n", 1);
    assert!(decode_ownership_manifest_v3(unknown.as_bytes()).is_err());
    let missing =
        canonical.replacen("  \"profile\": \"zryna-data-ownership-v1-candidate\",\n", "", 1);
    assert!(decode_ownership_manifest_v3(missing.as_bytes()).is_err());
    let reordered = canonical.replacen(
        "  \"version\": 3,\n  \"profile\": \"zryna-data-ownership-v1-candidate\",",
        "  \"profile\": \"zryna-data-ownership-v1-candidate\",\n  \"version\": 3,",
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
