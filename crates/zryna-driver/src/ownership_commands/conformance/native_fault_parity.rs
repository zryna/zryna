//! Current native parsing and bootstrap fault outcomes through the original owning phases.
//! Worker transport is qualified separately; this provider authenticates the original native DTO.

use super::super::{execute_with_fault, observation::Fault};
use super::*;
use std::collections::BTreeMap;
use zryna_frontend::{
    VerifiedFrontendProviderV4, WorkerError, native_lexer, native_parser, syntax_v4,
};
use zryna_source::{NormalizedSourcePath, SourceMap};

struct NativeParser;

impl VerifiedFrontendProviderV4 for NativeParser {
    fn analyze_verified_v4(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        let lexed = native_lexer::lex(sources).expect("frozen accepted native fault source tokens");
        let candidate = native_parser::v4::parse_v4_candidate(sources, &lexed)
            .expect("frozen accepted native fault source candidate");
        Ok(syntax_v4::verify_snapshot(candidate, sources)
            .expect("original mandatory source-bound native syntax verifier"))
    }
}

fn prepare_native(
    request: &DataOwnershipBuildRequest,
) -> crate::ownership_pipeline::DataOwnershipCandidateSuccess {
    let root = crate::WorkspaceSourceRoot::capture(&request.workspace_root)
        .expect("retained fault source root");
    let closure = crate::discover_ownership_module_closure(
        &root,
        NormalizedSourcePath::new(request.entrypoint.clone()).expect("fixed fault entrypoint"),
        &NativeParser,
    )
    .expect("native exact final source and syntax authority");
    let inputs = crate::pipeline::BuildInputs {
        workspace_root: request.workspace_root.clone(),
        entrypoint: request.entrypoint.clone(),
        artifact_stem: request.artifact_stem.clone(),
        targets: request.targets,
    };
    crate::ownership_pipeline::preparation::prepare_closure(
        &inputs,
        Some(request.node_runtime.clone()),
        closure,
        Some(("score".to_owned(), vec![])),
        &|_| Ok(()),
        &|| Ok(()),
    )
    .expect("original mandatory ownership IR, ABI and selected backend preparation")
}

fn observe(
    request: &DataOwnershipBuildRequest,
    native: bool,
    fault: Fault,
    oracle: &Value,
    physical: bool,
) -> (Value, BTreeMap<String, Vec<u8>>) {
    let prepared = if native {
        prepare_native(request)
    } else {
        prepare_data_ownership_for_test(request, Some(("score".to_owned(), vec![])))
            .expect("original authenticated bootstrap fault source")
    };
    let bundle = execute_with_fault(&prepared, Some(fault))
        .unwrap_or_else(|error| panic!("native={native} {}: {error:?}", oracle["id"]));
    assert_eq!(bundle.results().len(), 1, "one actual selected target observation");
    let result = &bundle.results()[0];
    let expected = if physical {
        serde_json::to_value(ScalarOutcome::Trapped { code: zryna_abi::ScalarTrapCode::Allocation })
            .expect("original physical allocation trap category")
    } else {
        oracle["expected"].clone()
    };
    assert_eq!(
        serde_json::to_value(result.outcome()).expect("typed observed outcome"),
        expected,
        "native={native} {}: independent original typed fault oracle",
        oracle["id"]
    );
    assert_eq!(
        serde_json::to_value(result.trace()).expect("complete observed cleanup trace"),
        oracle["trace"],
        "native={native} {}: independent original ordered cleanup oracle",
        oracle["id"]
    );
    let bytes = inventory(bundle.path());
    let manifest = crate::decode_ownership_manifest_v3(
        &fs::read(bundle.manifest_path()).expect("actual fault manifest bytes"),
    )
    .expect("original strict complete fault manifest decoder");
    assert_eq!(manifest.results(), bundle.results());
    let observations = serde_json::to_value(bundle.results()).expect("complete typed results");
    fs::remove_dir_all(bundle.path()).expect("remove only complete test-owned fault bundle");
    drop(prepared);
    assert_no_artifacts(&request.workspace_root);
    (observations, bytes)
}

#[test]
fn all_26_logical_faults_match_native_and_bootstrap_typed_cleanup_and_complete_bundles() {
    let _guard = route_guard();
    let registry = registry();
    let cases = registry["faults"].as_array().expect("original frozen logical fault inventory");
    assert_eq!(cases.len(), 26);
    let mut identities = std::collections::BTreeSet::new();
    let mut observations = 0;
    for case in cases {
        assert!(identities.insert(case["id"].as_str().expect("unique original fault id")));
        let fixture = case["fixture"].as_str().expect("original registered source");
        let code = u32::try_from(case["fault"]["code"].as_u64().expect("fixed fault code"))
            .expect("bounded fault code");
        let ordinal =
            u32::try_from(case["fault"]["ordinal"].as_u64().expect("fixed fault ordinal"))
                .expect("bounded fault ordinal");
        let targets = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            vec![TargetSelection::JavaScript, TargetSelection::WebAssembly, TargetSelection::Native]
        } else {
            // Windows native RUN has its separate N4002 rejection authority, never a fake run.
            vec![TargetSelection::JavaScript, TargetSelection::WebAssembly]
        };
        for target in targets {
            let workspace = fixture_workspace();
            install(workspace.root(), &registry, fixture);
            let request = request(workspace.root(), target);
            let fault = Fault::new(code, ordinal).expect("original bounded logical fault");
            let native = observe(&request, true, fault, case, false);
            let bootstrap = observe(&request, false, fault, case, false);
            assert_eq!(
                native, bootstrap,
                "{} {target:?}: every result and bundle byte",
                case["id"]
            );
            observations += 2;
        }
    }
    let targets = if cfg!(all(target_os = "linux", target_arch = "x86_64")) { 3 } else { 2 };
    assert_eq!(observations, 26 * targets * 2, "complete source, target and provider product");
    eprintln!(
        "native_fault_parity logical_cases=26 targets={targets} providers=2 observations={observations}"
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn all_12_physical_allocation_faults_match_native_and_bootstrap_cleanup_without_live_owners() {
    let _guard = route_guard();
    let registry = registry();
    let cases = [
        ("string", 2, "string-fault-2-1"),
        ("string", 4, "string-fault-2-2"),
        ("handles", 1, "handles-fault-2-1"),
        ("handles", 2, "handles-fault-2-1"),
        ("handles", 3, "handles-fault-4-1"),
        ("handles", 4, "handles-fault-4-2"),
        ("handles", 5, "handles-fault-4-3"),
        ("vec", 2, "vec-fault-2-1"),
        ("vec", 3, "vec-fault-2-2"),
        ("vec", 4, "vec-fault-2-3"),
        ("owned-shared", 4, "owned-shared-fault-2-2"),
        ("owned-vec", 10, "owned-vec-fault-2-6"),
    ];
    assert_eq!(cases.len(), 12, "unchanged physical allocation ordinal inventory");
    let mut observations = 0;
    for (fixture, ordinal, id) in cases {
        let oracle = registry["faults"]
            .as_array()
            .expect("original logical source cleanup oracle inventory")
            .iter()
            .find(|case| case["id"] == id)
            .expect("exact independently frozen physical cleanup oracle");
        let workspace = fixture_workspace();
        install(workspace.root(), &registry, fixture);
        let request = request(workspace.root(), TargetSelection::Native);
        let fault = Fault::physical_allocation(ordinal);
        let native = observe(&request, true, fault, oracle, true);
        let bootstrap = observe(&request, false, fault, oracle, true);
        assert_eq!(native, bootstrap, "{fixture} allocation{ordinal}: original physical cleanup");
        // The original native finalizer rejects a live allocation before publishing its frame.
        observations += 2;
    }
    assert_eq!(observations, 24, "two real executions for each physical allocation fault");
    eprintln!("native_fault_parity physical_cases=12 targets=1 providers=2 observations=24");
}
