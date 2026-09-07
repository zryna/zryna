use super::super::{execute_with_fault, observation::Fault};
use super::*;

#[test]
fn fixed_injected_faults_preserve_typed_outcomes_and_logical_cleanup() {
    let _guard = route_guard();
    let registry = registry();
    let cases = registry["faults"].as_array().expect("fixed fault corpus");
    assert_eq!(cases.len(), 26);
    let mut failures = Vec::new();
    for case in cases {
        let fixture = case["fixture"].as_str().unwrap();
        let code = u32::try_from(case["fault"]["code"].as_u64().unwrap()).unwrap();
        let ordinal = u32::try_from(case["fault"]["ordinal"].as_u64().unwrap()).unwrap();
        let expected: Vec<crate::OwnershipTraceEvent> =
            serde_json::from_value(case["trace"].clone()).unwrap();
        for target in
            [TargetSelection::JavaScript, TargetSelection::WebAssembly, TargetSelection::Native]
        {
            if target == TargetSelection::Native
                && !cfg!(all(target_os = "linux", target_arch = "x86_64"))
            {
                continue;
            }
            let workspace = fixture_workspace();
            install(workspace.root(), &registry, fixture);
            let prepared = prepare_data_ownership_for_test(
                &request(workspace.root(), target),
                Some(("score".to_owned(), vec![])),
            )
            .expect("source-valid fault fixture");
            let fault = Fault::new(code, ordinal).expect("bounded fault");
            match execute_with_fault(&prepared, Some(fault)) {
                Ok(bundle) => {
                    let result = &bundle.results()[0];
                    if serde_json::to_value(result.outcome()).unwrap() != case["expected"]
                        || result.trace() != expected
                    {
                        failures
                            .push(format!("{fixture} {code}@{ordinal} {target:?}: {:?}", result));
                    }
                    fs::remove_dir_all(bundle.path()).expect("remove complete observation bundle");
                }
                Err(error) => {
                    failures.push(format!("{fixture} {code}@{ordinal} {target:?}: {error:?}"))
                }
            }
            assert_no_artifacts(workspace.root());
        }
    }
    assert!(failures.is_empty(), "fault conformance failures:\n{}", failures.join("\n"));
}

#[test]
fn fault_selection_exact_limit_and_first_extra_are_request_owned() {
    assert!(Fault::new(2, 1_048_576).is_ok());
    for (code, ordinal) in [(1, 1), (6, 1), (2, 0), (2, 1_048_577)] {
        let error = match Fault::new(code, ordinal) {
            Err(error) => error,
            Ok(_) => panic!("invalid fault accepted"),
        };
        assert_eq!(error.kind(), CommandFailureKind::Request);
        assert_eq!(error.diagnostics()[0].code(), "ZRYNA-C3303");
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn native_physical_allocation_failures_release_intermediates_before_observation() {
    let _guard = route_guard();
    let registry = registry();
    // Physical ordinal counts descriptor and payload allocations separately.
    // References select independently frozen source cleanup plans, never another target's output.
    for (fixture, ordinal, oracle) in [
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
    ] {
        let workspace = fixture_workspace();
        install(workspace.root(), &registry, fixture);
        let prepared = prepare_data_ownership_for_test(
            &request(workspace.root(), TargetSelection::Native),
            Some(("score".to_owned(), vec![])),
        )
        .expect("physical fault source");
        let expected = registry["faults"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == oracle)
            .unwrap();
        let bundle = execute_with_fault(&prepared, Some(Fault::physical_allocation(ordinal)))
            .unwrap_or_else(|error| panic!("{fixture} allocation {ordinal}: {error:?}"));
        let result = &bundle.results()[0];
        assert_eq!(
            result.outcome(),
            ScalarOutcome::Trapped { code: zryna_abi::ScalarTrapCode::Allocation }
        );
        assert_eq!(
            serde_json::to_value(result.trace()).unwrap(),
            expected["trace"],
            "{fixture} allocation {ordinal}"
        );
        // Native harness finalization refuses any live owned allocation before emitting this frame.
        fs::remove_dir_all(bundle.path()).unwrap();
        assert_no_artifacts(workspace.root());
    }
}
