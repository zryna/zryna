#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use zryna_abi::{ScalarOutcome, ScalarValue};

use super::*;
use crate::{
    OwnershipTarget, TargetSelection,
    ownership_pipeline::{
        prepare_data_ownership_for_test,
        test_support::{fixture_workspace, node_executable, route_guard},
    },
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

fn request(root: &std::path::Path) -> DataOwnershipRunRequest {
    let sequence = NEXT_STEM.fetch_add(1, Ordering::Relaxed);
    DataOwnershipRunRequest {
        build: DataOwnershipBuildRequest {
            workspace_root: root.to_owned(),
            entrypoint: "main.zry".to_owned(),
            artifact_stem: format!("ownership-command-{}-{sequence}", std::process::id()),
            targets: TargetSelection::All,
            node_runtime: node_executable(),
        },
        logical_export: "score".to_owned(),
        arguments: vec![ScalarValue::I32(21)],
    }
}

#[test]
fn all_targets_execute_and_publish_one_typed_candidate_run() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let request = request(workspace.root());
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
