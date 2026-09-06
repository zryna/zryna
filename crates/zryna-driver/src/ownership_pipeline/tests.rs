use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use zryna_frontend::{VerifiedFrontendProviderV4, WorkerError, syntax_v4};
use zryna_source::SourceMap;

use super::test_support::{fixture_workspace, node_executable, route_guard};
use super::*;

#[derive(Clone)]
struct CountingFrontend {
    inner: WorkerFrontendV4,
    calls: Arc<AtomicUsize>,
}

impl VerifiedFrontendProviderV4 for CountingFrontend {
    fn minimum_analysis_timeout(&self) -> Duration {
        self.inner.minimum_analysis_timeout()
    }

    fn analyze_verified_v4(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.analyze_verified_v4(sources)
    }

    fn analyze_verified_v4_with_timeout(
        &self,
        sources: &SourceMap,
        timeout: Duration,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.analyze_verified_v4_with_timeout(sources, timeout)
    }
}

fn request(
    root: &std::path::Path,
    entrypoint: &str,
    targets: TargetSelection,
) -> DataOwnershipBuildRequest {
    DataOwnershipBuildRequest {
        workspace_root: root.to_owned(),
        entrypoint: entrypoint.to_owned(),
        artifact_stem: "candidate".to_owned(),
        targets,
        node_runtime: node_executable(),
    }
}

#[test]
fn one_final_authority_dispatches_all_targets_in_canonical_order() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let phase_observer = Arc::clone(&observed);
    let result = execute(
        &request(workspace.root(), "main.zry", TargetSelection::All),
        None,
        |request, node| {
            configured_test_frontend_v4(request, node)
                .map(|inner| CountingFrontend { inner, calls: Arc::clone(&calls) })
        },
        &move |phase| {
            phase_observer.lock().expect("phase lock").push(phase);
            Ok(())
        },
        validate_request_shape,
    )
    .expect("candidate build");

    assert_eq!(calls.load(Ordering::SeqCst), 3, "two discovery batches and one final analysis");
    assert_eq!(
        *observed.lock().expect("phase lock"),
        [DispatchPhase::JavaScript, DispatchPhase::WebAssembly, DispatchPhase::Native]
    );
    assert_eq!(
        result.program().verified_ir().source_map_identity(),
        result.closure().sources().identity()
    );
    assert_eq!(result.closure().modules().len(), 2);
    assert_eq!(result.closure().edges().len(), 1);
    assert!(result.artifacts().javascript().is_some());
    assert!(result.artifacts().webassembly().is_some());
    assert!(result.artifacts().native_object().is_some());
    assert!(result.artifacts().native_executable().is_none());
}

#[test]
fn typed_run_prepares_the_selected_native_executable() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let run = DataOwnershipRunRequest {
        build: request(workspace.root(), "main.zry", TargetSelection::Native),
        logical_export: "score".to_owned(),
        arguments: vec![ScalarValue::I32(21)],
    };
    let result = execute(
        &run.build,
        Some((run.logical_export, run.arguments)),
        configured_test_frontend_v4,
        &|_| Ok(()),
        validate_request_shape,
    )
    .expect("candidate run");

    let executable = result.artifacts().native_executable().expect("native executable");
    assert_eq!(result.logical_export(), Some("score"));
    assert_eq!(result.arguments(), [ScalarValue::I32(21)]);
    assert_eq!(executable.identity().source_map(), result.closure().sources().identity());
    assert_eq!(executable.identity().arguments(), result.arguments());
}

#[test]
fn invocation_mismatch_stops_before_backend_dispatch() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    let phases = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&phases);
    let failure = execute(
        &request(workspace.root(), "main.zry", TargetSelection::All),
        Some(("score".to_owned(), vec![ScalarValue::Bool(true)])),
        configured_test_frontend_v4,
        &move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        validate_request_shape,
    )
    .expect_err("wrong invocation must fail");

    assert_eq!(failure.kind(), CommandFailureKind::Source);
    assert_eq!(failure.diagnostics()[0].code(), "ZRYNA-B2103");
    assert_eq!(phases.load(Ordering::SeqCst), 0);
}
