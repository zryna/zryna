use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use zryna_abi::{ScalarOutcome, ScalarValue};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zryna_diagnostics::Diagnostic;

use super::*;
use crate::{
    DataOwnershipBuildRequest, TargetSelection,
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
            fs::remove_dir_all(&self.0).expect("test-owned bundle cleanup");
        }
    }
}

fn request(root: &Path, targets: TargetSelection) -> DataOwnershipBuildRequest {
    let sequence = NEXT_STEM.fetch_add(1, Ordering::Relaxed);
    DataOwnershipBuildRequest {
        workspace_root: root.to_owned(),
        entrypoint: "main.zry".to_owned(),
        artifact_stem: format!("ownership-publication-{}-{sequence}", std::process::id()),
        targets,
        node_runtime: node_executable(),
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn transaction_names(output: &Path) -> Vec<String> {
    let mut names = fs::read_dir(output)
        .expect("output directory")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(".zryna-transaction-"))
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn failures_rollback_then_complete_bundle_publishes_create_only() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let request = request(workspace.root(), TargetSelection::All);
    let success = prepare_data_ownership_for_test(&request, None).expect("candidate build");
    let output =
        ArtifactOutputRoot::prepare_for_workspace(&request.workspace_root).expect("output");
    let final_path = output.path().join(format!("{}.build", request.artifact_stem));
    let _cleanup = BundleCleanup(final_path.clone());
    let baseline = transaction_names(output.path());

    for injected in [
        PublicationPhase::JavaScript,
        PublicationPhase::WebAssembly,
        PublicationPhase::Native,
        PublicationPhase::Manifest,
        PublicationPhase::Commit,
    ] {
        let failure = publish_with_checkpoint(&success, &|phase| {
            if phase == injected {
                Err(CommandFailure {
                    kind: CommandFailureKind::Preparation,
                    diagnostics: vec![Diagnostic::error(
                        "ZRYNA-C3299",
                        None,
                        "test-injected ownership publication failure",
                        "verify transaction rollback",
                    )],
                })
            } else {
                Ok(())
            }
        })
        .expect_err("injected publication failure");
        assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C3299");
        assert!(!final_path.exists());
        assert_eq!(transaction_names(output.path()), baseline);
    }

    let published = publish_data_ownership_build(&success).expect("complete bundle");
    assert_eq!(published.path(), final_path);
    assert_eq!(published.artifacts().len(), 3);
    assert!(published.artifacts().iter().all(|artifact| artifact.path().is_file()));
    let manifest_before = fs::read(published.manifest_path()).expect("manifest bytes");
    decode_ownership_manifest_v3(&manifest_before).expect("strict published manifest");

    let collision = publish_data_ownership_build(&success).expect_err("create-only collision");
    assert_eq!(collision.kind(), CommandFailureKind::Preparation);
    assert_eq!(fs::read(published.manifest_path()).expect("retained manifest"), manifest_before);
    assert_eq!(transaction_names(output.path()), baseline);
}

#[test]
fn complete_run_bundle_binds_typed_results() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let request = request(workspace.root(), TargetSelection::JavaScript);
    let success = prepare_data_ownership_for_test(
        &request,
        Some(("score".to_owned(), vec![ScalarValue::I32(21)])),
    )
    .expect("candidate run");
    let result = OwnershipManifestResult::new(
        OwnershipTarget::JavaScript,
        ScalarOutcome::Returned { value: ScalarValue::I32(42) },
    );
    let published = publish_after_staging(&success, |_, _| Ok(vec![result])).expect("run bundle");
    let _cleanup = BundleCleanup(published.path().to_owned());

    assert_eq!(published.command(), CommandKind::Run);
    assert_eq!(published.results(), [result]);
    assert!(published.path().ends_with(format!("{}.run", request.artifact_stem)));
    let manifest = fs::read(published.manifest_path()).expect("manifest bytes");
    let decoded = decode_ownership_manifest_v3(&manifest).expect("strict manifest");
    assert_eq!(decoded.results(), [result]);
}

#[test]
fn build_publication_rejects_prepared_run_without_execution_evidence() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let request = request(workspace.root(), TargetSelection::JavaScript);
    let success = prepare_data_ownership_for_test(
        &request,
        Some(("score".to_owned(), vec![ScalarValue::I32(21)])),
    )
    .expect("candidate run");
    let failure = publish_data_ownership_build(&success).expect_err("unexecuted run must fail");

    assert_eq!(failure.kind(), CommandFailureKind::Preparation);
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-C3201");
    assert!(
        !request.workspace_root.join(format!(".zryna/out/{}.run", request.artifact_stem)).exists()
    );
}
