//! Issue #377: private A1–A5 source, trap, cleanup, and storage observations.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};
use zryna_abi::{Invocation, ScalarValue};

use super::{execute_with_fault, observation::Fault};
use crate::{
    CommandFailureKind, DataOwnershipBuildRequest, TargetSelection,
    ownership_pipeline::{
        prepare_data_ownership_for_test,
        test_support::{fixture_workspace, node_executable, route_guard},
    },
    runtime::NodeRuntimeCapability,
};

mod capacity;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod native;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m4-fixtures/allocation-core")
}

fn cases() -> Vec<Value> {
    serde_json::from_slice(&fs::read(corpus().join("cases.json")).expect("allocation cases"))
        .expect("fixed allocation oracles")
}

fn request(root: &std::path::Path, targets: TargetSelection) -> DataOwnershipBuildRequest {
    DataOwnershipBuildRequest {
        workspace_root: root.to_owned(),
        entrypoint: "main.zry".to_owned(),
        artifact_stem: "allocation-core".to_owned(),
        targets,
        node_runtime: node_executable(),
    }
}

fn install(root: &std::path::Path, name: &str) {
    fs::copy(corpus().join(format!("{name}.zry")), root.join("main.zry"))
        .expect("isolated allocation source");
    let dependency = format!("{name}-body.zry");
    if corpus().join(&dependency).is_file() {
        fs::copy(corpus().join(&dependency), root.join(dependency))
            .expect("isolated allocation dependency");
    }
}

fn arguments(case: &Value) -> Vec<ScalarValue> {
    case["arguments"]
        .as_array()
        .expect("fixed scalar arguments")
        .iter()
        .map(|value| {
            ScalarValue::I32(i32::try_from(value.as_i64().expect("integer")).expect("i32"))
        })
        .collect()
}

fn fault(case: &Value) -> Fault {
    let (code, ordinal) = case["fault"].as_array().map_or((2, 1_048_576), |value| {
        (value[0].as_u64().expect("code"), value[1].as_u64().expect("ordinal"))
    });
    Fault::new(u32::try_from(code).expect("code"), u32::try_from(ordinal).expect("ordinal"))
        .expect("bounded private fault")
}

fn expected_outcome(case: &Value) -> Value {
    if case["result"].is_i64() {
        json!({"kind": "returned", "value": {"type": "i32", "value": case["result"]}})
    } else {
        json!({"kind": "trapped", "code": format!("zryna.trap.{}-v1", case["result"].as_str().expect("trap"))})
    }
}

fn run_node_inspection(script: &Path, root: &Path) -> Result<Vec<u8>, String> {
    let runtime = NodeRuntimeCapability::discover(&node_executable(), root)
        .map_err(|error| format!("private runtime discovery: {error}"))?;
    runtime
        .run_ownership_javascript(script, root)
        .map_err(|error| format!("private bounded inspection: {error}"))
}

fn check_case(case: &Value, target: TargetSelection) -> Result<(), String> {
    let workspace = fixture_workspace();
    install(workspace.root(), case["fixture"].as_str().expect("source"));
    let prepared = prepare_data_ownership_for_test(
        &request(workspace.root(), target),
        Some(("score".to_owned(), arguments(case))),
    )
    .map_err(|error| format!("prepare: {error:?}"))?;
    let bundle = execute_with_fault(&prepared, Some(fault(case)))
        .map_err(|error| format!("execute: {error:?}"))?;
    let result = &bundle.results()[0];
    let actual = serde_json::to_value(result.outcome()).expect("typed outcome");
    if actual != expected_outcome(case) {
        return Err(format!("outcome: {actual}; expected {}", expected_outcome(case)));
    }
    if let Some(cleanup) = case["cleanup"].as_array() {
        let expected: Vec<Value> = cleanup
            .iter()
            .flat_map(|row| {
                [
                    json!({"kind": "cleanup", "module": case["module"].as_u64().unwrap_or(0), "function": 0, "place": row[0]}),
                    json!({"kind": "drop", "value": row[1]}),
                ]
            })
            .collect();
        let actual = serde_json::to_value(result.trace()).expect("logical trace");
        if actual != json!(expected) {
            return Err(format!("cleanup: {actual}; expected {}", json!(expected)));
        }
    }
    if case["inspect"].as_bool() == Some(true) {
        let invocation = prepared
            .program()
            .verified_ir()
            .scalar_abi()
            .prepare_invocation(Invocation::new("score".to_owned(), arguments(case)))
            .expect("verified scalar entry");
        let (kind, artifact, export) = match target {
            TargetSelection::JavaScript => (
                "javascript",
                prepared.artifacts().javascript().expect("JavaScript").source.as_bytes(),
                invocation.export().javascript_name().as_str(),
            ),
            TargetSelection::WebAssembly => (
                "webassembly",
                prepared.artifacts().webassembly().expect("WebAssembly").bytes(),
                invocation.export().webassembly_name().as_str(),
            ),
            _ => return Ok(()),
        };
        let layouts = prepared.program().verified_ir().linear32_layouts();
        let category = match case["storage"].as_str().expect("inspection storage") {
            "string" => zryna_layout::TypeCategory::String,
            "vec" => zryna_layout::TypeCategory::Vec,
            other => return Err(format!("unsupported inspection storage: {other}")),
        };
        let ty = layouts
            .types()
            .find(|ty| ty.category() == category)
            .ok_or_else(|| format!("missing {category:?} inspection layout"))?;
        let drop_index = 2_u32
            .checked_add(u32::try_from(layouts.types().len()).expect("bounded type count"))
            .and_then(|index| index.checked_add(ty.id().index()))
            .expect("bounded drop helper index");
        let path = workspace.root().join("inspection-input");
        fs::write(&path, artifact).expect("private inspection input");
        fs::write(
            workspace.root().join("allocation-inspection.json"),
            serde_json::to_vec(&json!({
                "target": kind,
                "entry": export,
                "id": case["id"].as_str().expect("case id"),
                "dropIndex": drop_index,
            }))
            .expect("private inspection command"),
        )
        .expect("private inspection command");
        let output = run_node_inspection(&corpus().join("inspect.mjs"), workspace.root())?;
        if output != b"allocation observation passed\n" {
            return Err("missing complete private observation".to_owned());
        }
    }
    Ok(())
}

fn check_target(target: TargetSelection) {
    let _guard = route_guard();
    let mut failures = Vec::new();
    for case in cases() {
        if let Err(error) = check_case(&case, target) {
            failures.push(format!("{} {target:?}: {error}", case["id"]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn allocation_core_javascript_fixed_results_faults_and_cleanup() {
    check_target(TargetSelection::JavaScript);
}

#[test]
fn allocation_core_webassembly_fixed_results_faults_and_cleanup() {
    check_target(TargetSelection::WebAssembly);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn allocation_core_native_fixed_results_faults_and_cleanup() {
    check_target(TargetSelection::Native);
}

#[test]
fn allocation_core_n7_n10_reject_before_target_dispatch() {
    let _guard = route_guard();
    let negatives: Vec<Value> =
        serde_json::from_slice(&fs::read(corpus().join("negatives.json")).expect("negative cases"))
            .expect("fixed negative oracles");
    let mut failures = Vec::new();
    for case in negatives {
        for target in
            [TargetSelection::JavaScript, TargetSelection::WebAssembly, TargetSelection::Native]
        {
            let workspace = fixture_workspace();
            install(workspace.root(), case["fixture"].as_str().expect("source"));
            match prepare_data_ownership_for_test(&request(workspace.root(), target), None) {
                Err(error) if error.kind() == CommandFailureKind::Source => {
                    let diagnostic = &error.diagnostics()[0];
                    if diagnostic.code() != case["code"].as_str().expect("stable code")
                        || diagnostic.primary_span().is_none()
                    {
                        failures.push(format!("{} {target:?}: {error:?}", case["fixture"]));
                    }
                }
                other => failures.push(format!("{} {target:?}: {other:?}", case["fixture"])),
            }
            assert!(
                !workspace.root().join(".zryna/out").exists(),
                "rejected source reached publication"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn allocation_core_native_run_requires_supported_linux_host() {
    let _guard = route_guard();
    let workspace = fixture_workspace();
    install(workspace.root(), "vec-push");
    let error = prepare_data_ownership_for_test(
        &request(workspace.root(), TargetSelection::Native),
        Some(("score".to_owned(), vec![])),
    )
    .expect_err("native execution requires Linux x86-64");
    assert_eq!(error.kind(), CommandFailureKind::Preparation);
    assert_eq!(error.diagnostics()[0].code(), "ZRYNA-N4002");
    let output = workspace.root().join(".zryna/out");
    if output.exists() {
        assert_eq!(fs::read_dir(output).expect("output inventory").count(), 0);
    }
}
