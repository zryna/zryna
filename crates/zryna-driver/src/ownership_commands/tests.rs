use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use zryna_abi::{ScalarOutcome, ScalarValue};

use super::*;
use crate::{
    TargetSelection,
    ownership_pipeline::{OWNERSHIP_ROUTE_TEST_LOCK, prepare_data_ownership_for_test},
};

static NEXT_STEM: AtomicUsize = AtomicUsize::new(0);

struct BundleCleanup(PathBuf);

impl Drop for BundleCleanup {
    fn drop(&mut self) {
        if self.0.exists() {
            fs::remove_dir_all(&self.0).expect("test-owned candidate bundle cleanup");
        }
    }
}

fn request() -> DataOwnershipRunRequest {
    let sequence = NEXT_STEM.fetch_add(1, Ordering::Relaxed);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root");
    DataOwnershipRunRequest {
        build: DataOwnershipBuildRequest {
            workspace_root: root,
            entrypoint: "tests/m3-fixtures/candidate-modules/main.zry".to_owned(),
            artifact_stem: format!("ownership-command-{}-{sequence}", std::process::id()),
            targets: TargetSelection::All,
            node_runtime: PathBuf::from("/usr/bin/node"),
        },
        logical_export: "score".to_owned(),
        arguments: vec![ScalarValue::I32(21)],
    }
}

#[test]
fn all_targets_execute_and_publish_one_typed_candidate_run() {
    let _guard = OWNERSHIP_ROUTE_TEST_LOCK.lock().expect("route test lock");
    let request = request();
    let success = prepare_data_ownership_for_test(
        &request.build,
        Some((request.logical_export, request.arguments)),
    )
    .expect("prepared candidate run");
    let published = execute_and_publish_run(&success).expect("executed candidate run");
    let _cleanup = BundleCleanup(published.path().to_owned());

    assert_eq!(published.results().len(), 3);
    for (result, target) in published.results().iter().zip([
        OwnershipTarget::JavaScript,
        OwnershipTarget::WebAssembly,
        OwnershipTarget::Native,
    ]) {
        assert_eq!(result.target(), target);
        assert_eq!(result.outcome(), ScalarOutcome::Returned { value: ScalarValue::I32(42) });
    }
    assert!(published.manifest_path().is_file());
    crate::decode_ownership_manifest_v3(
        &fs::read(published.manifest_path()).expect("manifest bytes"),
    )
    .expect("strict published manifest");
}
