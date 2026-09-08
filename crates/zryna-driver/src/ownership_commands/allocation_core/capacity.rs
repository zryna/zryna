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
        // Every loop iteration has derived temporary cleanup, so tracing the
        // exact million-element boundary would exceed the fixed 4,096-word
        // observation frame. The small corpus independently proves owner count
        // and reverse cleanup; these rows retain the bounded frame and isolate
        // exact/first-extra target outcomes.
        let bundle = execute_with_fault(&prepared, None)
            .unwrap_or_else(|error| panic!("{} {target:?} execute: {error:?}", case["id"]));
        assert_eq!(bundle.results().len(), 1);
        let result = &bundle.results()[0];
        assert_eq!(
            serde_json::to_value(result.outcome()).expect("typed capacity outcome"),
            expected_outcome(case),
            "{} {target:?}",
            case["id"]
        );
        assert!(result.trace().is_empty(), "{} {target:?} trace isolation", case["id"]);
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
    let output = run_node_inspection(&corpus().join("capacity-inspect.mjs"), workspace.root())
        .expect("private allocator inspection");
    assert_eq!(output, b"allocation capacity observation passed\n");
}
