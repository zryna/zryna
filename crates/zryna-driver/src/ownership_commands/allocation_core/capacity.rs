//! Resource cases kept separate from the small Q4–Q8 fixture loop.

use super::*;

fn capacity_cases() -> Value {
    serde_json::from_slice(&fs::read(corpus().join("capacity-cases.json")).expect("capacity cases"))
        .expect("fixed capacity oracles")
}

fn source_boundaries(target: TargetSelection) {
    let _guard = route_guard();
    let cases = capacity_cases();
    for case in cases["source"].as_array().expect("source capacity cases") {
        let workspace = fixture_workspace();
        install(workspace.root(), case["fixture"].as_str().expect("source fixture"));
        let prepared = prepare_data_ownership_for_test(
            &request(workspace.root(), target),
            Some(("score".to_owned(), arguments(case))),
        )
        .unwrap_or_else(|error| panic!("{} {target:?} prepare: {error:?}", case["id"]));
        // These fixtures contain no String operation. An unreachable UTF-8 fault
        // enables cleanup tracing without failing the millionth allocation probe.
        let bundle = execute_with_fault(&prepared, Some(Fault::new(5, 1).expect("trace command")))
            .unwrap_or_else(|error| panic!("{} {target:?} execute: {error:?}", case["id"]));
        assert_eq!(bundle.results().len(), 1);
        let result = &bundle.results()[0];
        assert_eq!(
            serde_json::to_value(result.outcome()).expect("typed capacity outcome"),
            expected_outcome(case),
            "{} {target:?}",
            case["id"]
        );
        let trace = serde_json::to_value(result.trace()).expect("capacity cleanup");
        let trace = trace.as_array().expect("cleanup events");
        let owners = usize::try_from(case["cleanupOwners"].as_u64().expect("owner count"))
            .expect("bounded count");
        assert_eq!(trace.len(), 2 * owners, "{} {target:?}", case["id"]);
        let mut previous = u64::MAX;
        for pair in trace.chunks_exact(2) {
            let place = pair[0]["place"].as_u64().expect("cleanup place");
            // These fixtures declare one owner, then optionally its clone. Place
            // identities increase with declaration order; both must drop once,
            // newest first, without depending on scalar loop temporary numbers.
            assert!(place < previous, "{} {target:?} reverse owner order", case["id"]);
            previous = place;
            assert_eq!(
                pair[0],
                json!({"kind": "cleanup", "module": 0, "function": 0, "place": place})
            );
            assert_eq!(pair[1], json!({"kind": "drop", "value": "sequence"}));
        }
    }
}

#[test]
fn allocation_core_capacity_javascript_source_maximum_and_first_extra() {
    source_boundaries(TargetSelection::JavaScript);
}

#[test]
fn allocation_core_capacity_webassembly_source_maximum_and_first_extra() {
    source_boundaries(TargetSelection::WebAssembly);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn allocation_core_capacity_native_source_maximum_and_first_extra() {
    source_boundaries(TargetSelection::Native);
}

#[test]
fn allocation_core_capacity_webassembly_private_allocator_boundaries() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    install(workspace.root(), "vec-push");
    let prepared = prepare_data_ownership_for_test(
        &request(workspace.root(), TargetSelection::WebAssembly),
        None,
    )
    .expect("verified allocation fixture");
    let path = workspace.root().join("allocation-boundaries.wasm");
    fs::write(&path, prepared.artifacts().webassembly().expect("WebAssembly").bytes())
        .expect("private allocator input");
    let output = Command::new(node_executable())
        .arg(corpus().join("capacity-inspect.mjs"))
        .arg(path)
        .output()
        .expect("private allocator inspection");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"allocation capacity observation passed\n");
}
