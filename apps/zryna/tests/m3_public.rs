//! Exact public profile observations and shared-driver equivalence.
#![forbid(unsafe_code)]

#[path = "m3_public/support.rs"]
mod support;

use serde_json::{Value, json};
use std::{fs, process::Command};
use support::{Case, inventory, invocation, registry, successful};
use zryna_abi::ScalarValue;
use zryna_driver::{DataOwnershipRunRequest, TargetSelection};

#[test]
fn public_beginner_corrections_execute_complete_published_sources() {
    let _guard = support::guard();
    let guide = include_str!("../../../docs/M3_GETTING_STARTED.md");
    for (id, expected) in [("corrected-move", 17), ("corrected-name", 7)] {
        let section =
            guide.split(&format!("### {id}\n")).nth(1).expect("public M3 fixture invariant");
        let section = section.split("\n##").next().expect("public M3 fixture invariant");
        let sources = section
            .split("```zry\n")
            .skip(1)
            .map(|block| block.split("```").next().expect("public M3 fixture invariant"))
            .collect::<Vec<_>>();
        assert_eq!(sources.len(), if id == "corrected-move" { 2 } else { 1 });
        let case = Case::new("pair");
        fs::write(case.root.join(&case.source), sources[0]).expect("public M3 fixture invariant");
        if let Some(dependency) = sources.get(1) {
            fs::write(case.root.join(&case.source).with_file_name("math.zry"), dependency)
                .expect("public M3 fixture invariant");
        }
        assert!(section.contains(&format!("--name m3-{id}-1 --export score --node \"$NODE\"")));
        for target in ["javascript", "webassembly", "native"] {
            if target == "native" && !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
                continue;
            }
            let result = successful(&case.run("run", target, &["--export".into(), "score".into()]));
            assert_eq!(
                result["results"][0]["outcome"],
                json!({"kind":"returned", "value":{"type":"i32", "value":expected}})
            );
            case.clear("run");
        }
    }
}

#[test]
fn public_fixed_corpus_matches_oracles_and_candidate_bytes() {
    let _guard = support::guard();
    let registry = registry();
    assert_eq!(registry["valid"].as_array().expect("public M3 fixture invariant").len(), 15);
    for entry in registry["valid"].as_array().expect("public M3 fixture invariant") {
        let case = Case::new(entry["fixture"].as_str().expect("public M3 fixture invariant"));
        let selections = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            vec![("all", TargetSelection::All)]
        } else {
            vec![
                ("javascript", TargetSelection::JavaScript),
                ("webassembly", TargetSelection::WebAssembly),
            ]
        };
        for (target, selection) in selections {
            let response = successful(&case.run("run", target, &invocation(entry)));
            let expected =
                json!({"kind":"returned", "value":{"type":"i32", "value":entry["expected"]}});
            let results = response["results"].as_array().expect("public M3 fixture invariant");
            assert_eq!(results.len(), if target == "all" { 3 } else { 1 });
            for result in results {
                assert_eq!(result["outcome"], expected, "{}", entry["id"]);
            }
            let bytes = inventory(&case.bundle("run"));
            let manifest: Value = serde_json::from_slice(&bytes["zryna-manifest-v3.json"])
                .expect("public M3 fixture invariant");
            assert_eq!(manifest["version"], 3);
            assert_eq!(manifest["profile"], "zryna-data-ownership-v1");
            assert_eq!(manifest["protocol_version"], 4);
            assert_eq!(manifest["results"], response["results"]);
            zryna_driver::decode_ownership_manifest_v3(&bytes["zryna-manifest-v3.json"])
                .expect("public M3 fixture invariant");
            case.clear("run");
            let candidate = zryna_driver::run_data_ownership_candidate(DataOwnershipRunRequest {
                build: case.request(selection),
                logical_export: entry["export"]
                    .as_str()
                    .expect("public M3 fixture invariant")
                    .to_owned(),
                arguments: entry["arguments"]
                    .as_array()
                    .expect("public M3 fixture invariant")
                    .iter()
                    .map(|arg| {
                        ScalarValue::I32(
                            i32::try_from(arg.as_i64().expect("public M3 fixture invariant"))
                                .expect("public M3 fixture invariant"),
                        )
                    })
                    .collect(),
            })
            .expect("public M3 fixture invariant");
            assert_eq!(inventory(candidate.path()), bytes, "{} public/candidate", entry["id"]);
            case.clear("run");
        }
        successful(&case.run("build", "all", &[]));
        let bytes = inventory(&case.bundle("build"));
        assert_eq!(bytes.len(), 4);
        case.clear("build");
        let candidate =
            zryna_driver::build_data_ownership_candidate(&case.request(TargetSelection::All))
                .expect("public M3 fixture invariant");
        assert_eq!(inventory(candidate.path()), bytes, "{} build equivalence", entry["id"]);
    }
}

#[test]
fn public_invalid_corpus_rejects_without_artifacts() {
    let _guard = support::guard();
    let registry = registry();
    for entry in registry["invalid"].as_array().expect("public M3 fixture invariant") {
        let case = Case::new(entry["fixture"].as_str().expect("public M3 fixture invariant"));
        let (command, args) = if entry["export"].is_string() {
            ("run", invocation(entry))
        } else {
            ("build", vec![])
        };
        let output = case.run(command, "all", &args);
        assert_eq!(output.status.code(), Some(3));
        let response: Value =
            serde_json::from_slice(&output.stdout).expect("public M3 fixture invariant");
        assert_eq!(response["ok"], false);
        assert_eq!(response["manifest"], Value::Null);
        assert_eq!(response["diagnostics"][0]["code"], entry["code"]);
        assert!(!case.bundle(command).exists());
    }
}

#[test]
fn public_typed_bounds_traps_match_candidate_cleanup_and_manifest() {
    let _guard = support::guard();
    let registry = registry();
    for entry in registry["runtimeInvalid"].as_array().expect("public M3 fixture invariant") {
        let case = Case::new(entry["fixture"].as_str().expect("public M3 fixture invariant"));
        for (target, selection) in [
            ("javascript", TargetSelection::JavaScript),
            ("webassembly", TargetSelection::WebAssembly),
            ("native", TargetSelection::Native),
        ] {
            if target == "native" && !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
                continue;
            }
            let response = successful(&case.run("run", target, &invocation(entry)));
            assert_eq!(
                response["results"][0]["outcome"],
                json!({"kind":"trapped", "code":entry["expectedTrap"]})
            );
            let before = inventory(&case.bundle("run"));
            case.clear("run");
            let candidate = zryna_driver::run_data_ownership_candidate(DataOwnershipRunRequest {
                build: case.request(selection),
                logical_export: entry["export"]
                    .as_str()
                    .expect("public M3 fixture invariant")
                    .to_owned(),
                arguments: entry["arguments"]
                    .as_array()
                    .expect("public M3 fixture invariant")
                    .iter()
                    .map(|arg| {
                        ScalarValue::I32(
                            i32::try_from(arg.as_i64().expect("public M3 fixture invariant"))
                                .expect("public M3 fixture invariant"),
                        )
                    })
                    .collect(),
            })
            .expect("public M3 fixture invariant");
            assert_eq!(inventory(candidate.path()), before);
            case.clear("run");
        }
    }
}

#[test]
fn public_atomic_rerun_collision_and_exact_stem_boundary() {
    let _guard = support::guard();
    let mut case = Case::new("pair");
    for command in ["build", "run"] {
        let args = if command == "run" {
            vec!["--export".into(), "score".into(), "--arg=i32:2".into(), "--arg=i32:4".into()]
        } else {
            vec![]
        };
        successful(&case.run(command, "javascript", &args));
        let before = inventory(&case.bundle(command));
        let collision = case.run(command, "javascript", &args);
        assert!(!collision.status.success());
        assert_eq!(inventory(&case.bundle(command)), before);
        case.clear(command);
        fs::write(case.bundle(command), b"existing destination")
            .expect("public M3 fixture invariant");
        assert!(!case.run(command, "javascript", &args).status.success());
        assert_eq!(
            fs::read(case.bundle(command)).expect("public M3 fixture invariant"),
            b"existing destination"
        );
        fs::remove_file(case.bundle(command)).expect("public M3 fixture invariant");
        successful(&case.run(command, "javascript", &args));
        assert_eq!(inventory(&case.bundle(command)), before);
        case.clear(command);
    }
    let original = case.stem.clone();
    case.stem =
        format!("{original}{}", "a".repeat(zryna_driver::MAX_ARTIFACT_STEM_BYTES - original.len()));
    successful(&case.run("build", "javascript", &[]));
    case.clear("build");
    case.stem.push('a');
    let rejected = case.run("build", "javascript", &[]);
    assert_eq!(rejected.status.code(), Some(2));
    let response: Value =
        serde_json::from_slice(&rejected.stdout).expect("public M3 fixture invariant");
    assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-D2001");
    assert!(!case.bundle("build").exists());
    case.stem = original;
}

#[test]
fn public_exact_spellings_boolean_arguments_and_unsupported_selectors() {
    let _guard = support::guard();
    let case = Case::new("pair");
    fs::write(
        case.root.join(&case.source),
        "export function invert(value: bool): bool { return value; }",
    )
    .expect("public M3 fixture invariant");
    for spelling in [vec!["--profile", "data-ownership-v1"], vec!["--profile=data-ownership-v1"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_zryna"))
            .args([
                "run",
                &case.source,
                "--target",
                "javascript",
                "--name",
                &case.stem,
                "--export",
                "invert",
                "--arg=bool:true",
                "--json",
            ])
            .args(spelling)
            .arg("--root")
            .arg(&case.root)
            .arg("--node")
            .arg(&case.node)
            .output()
            .expect("public M3 fixture invariant");
        let response = successful(&output);
        assert_eq!(
            response["results"][0]["outcome"]["value"],
            json!({"type":"bool", "value":true})
        );
        case.clear("run");
    }
    for profile in ["m3", "DataOwnershipV1", "data-ownership-v1-candidate", "data-ownership-v2"] {
        let output = Command::new(env!("CARGO_BIN_EXE_zryna"))
            .args(["build", &case.source, "--profile", profile, "--target", "javascript", "--root"])
            .arg(&case.root)
            .arg("--node")
            .arg(&case.node)
            .output()
            .expect("public M3 fixture invariant");
        assert_eq!(output.status.code(), Some(2));
    }
    for target in ["wasi", "components", "windows", "browser"] {
        assert_eq!(case.run("build", target, &[]).status.code(), Some(2));
    }
    fs::write(
        case.root.join(&case.source),
        "export function bad(value: String): String { return value; }",
    )
    .expect("public M3 fixture invariant");
    assert!(!case.run("build", "javascript", &[]).status.success());
    assert!(!case.bundle("build").exists());
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn public_native_run_rejects_unsupported_host() {
    let _guard = support::guard();
    let case = Case::new("array");
    let output = case.run("run", "native", &["--export".into(), "score".into()]);
    assert!(!output.status.success());
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("public M3 fixture invariant");
    assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-N4002");
    assert!(!case.bundle("run").exists());
}
