//! Public generic profile controls frozen before activation; no private emitter substitutes.
#![forbid(unsafe_code)]
#[path = "generic_public/negative.rs"]
mod negative;
#[path = "generic_public/support.rs"]
mod support;
use serde_json::{Value, json};
use std::{fs, process::Output};
use support::{Case, inventory, manifest, output, pins};

#[test]
fn native_frontend_builds_all_ordinary_targets_without_node_or_linker() {
    let _guard = support::guard();
    let case = Case::new("scalar");
    let mut command = case.command("build", "all", false, "native", None);
    command.env("PATH", "").env_remove("NODE").env_remove("ZRYNA_TEST_NODE");
    let response = successful(&output(&mut command), &case, "build");
    assert_eq!(response["results"], json!([]));
    let metadata = manifest(&case, "build");
    assert_eq!(metadata["source"]["entryModule"], 0);
    assert_eq!(metadata["artifacts"].as_array().expect("ordinary artifacts").len(), 3);
    assert_eq!(inventory(&case.bundle("build")).len(), 4);
}

#[test]
fn original_template_only_library_builds_all_with_empty_scalar_abi() {
    let _guard = support::guard();
    let case = Case::new("scalar");
    let frozen = pins();
    let row = &frozen["fixtures"]["entry-one"]["files"][0];
    assert_eq!(row["member"], "a.zry");
    let original = support::original(row);
    assert_eq!(original.len(), 80);
    assert_eq!(row["sha256"], "b0b189e867c2bbdac4fd226e525636bdb3a37496213d4ce913593f0f69c3615b");
    fs::write(case.root.join(&case.source), original).expect("exact original library input");
    let mut command = case.command("build", "all", false, "native", None);
    command.env("PATH", "").env_remove("NODE").env_remove("ZRYNA_TEST_NODE");
    let response = successful(&output(&mut command), &case, "build");
    assert_eq!(response["results"], json!([]));
    let metadata = manifest(&case, "build");
    assert_eq!(metadata["exports"], json!([]));
    assert_eq!(metadata["invocation"], Value::Null);
    assert_eq!(metadata["source"]["entryModule"], 0);
    assert_eq!(metadata["source"]["edges"], json!([]));
    let modules = metadata["source"]["modules"].as_array().expect("sole source module");
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0]["bytes"], 80);
    assert_eq!(modules[0]["sha256"], row["sha256"]);
    assert_eq!(metadata["artifacts"].as_array().expect("all ordinary targets").len(), 3);
    let javascript = fs::read_to_string(
        case.bundle("build").join("javascript").join(format!("{}.mjs", case.stem)),
    )
    .expect("ordinary JavaScript library");
    // Private function indices are dense: a zero-demand inventory cannot emit $g0.
    assert!(!javascript.contains("function $g0("));
    assert!(javascript.ends_with("export {};\n"));
}

#[test]
fn exact_selector_spellings_run_original_scalar_entry_one_mixed_and_bool_clone() {
    let _guard = support::guard();
    let node = support::node();
    for fixture in ["scalar", "entry-one", "mixed", "bool-clone"] {
        let case = Case::new(fixture);
        for equals in [false, true] {
            for target in targets() {
                let mut command = case.command("run", target, equals, "native", Some(&node));
                command.args(["--export", "entry"]);
                let response = successful(&output(&mut command), &case, "run");
                let records = response["results"].as_array().expect("real target outcomes");
                assert_eq!(records.len(), if *target == "all" { 3 } else { 1 });
                let expected_targets = if *target == "all" {
                    vec!["javascript", "webassembly", "native"]
                } else {
                    vec![*target]
                };
                assert_eq!(
                    records
                        .iter()
                        .map(|row| row["target"].as_str().expect("target name"))
                        .collect::<Vec<_>>(),
                    expected_targets
                );
                for record in records {
                    assert_eq!(
                        record["outcome"],
                        json!({"kind":"returned", "value":{
                        "type":"i32", "value":pins()["fixtures"][fixture]["result_i32"]}})
                    );
                }
                let metadata = manifest(&case, "run");
                assert_eq!(
                    metadata["source"]["entryModule"],
                    pins()["fixtures"][fixture]["entry_module"]
                );
                assert_eq!(metadata["invocation"]["outcomes"], response["results"]);
                case.clear("run");
            }
        }
    }
}

#[test]
fn all_three_providers_replay_same_stem_artifacts_and_strict_manifest_bytes() {
    let _guard = support::guard();
    let node = support::node();
    for fixture in ["scalar", "entry-one", "mixed", "bool-clone"] {
        let case = Case::new(fixture);
        let mut baseline = None;
        for provider in ["native", "bootstrap", "compare"] {
            let selected_node = (provider != "native").then_some(node.as_path());
            successful(
                &output(&mut case.command("build", "all", false, provider, selected_node)),
                &case,
                "build",
            );
            manifest(&case, "build");
            let actual = inventory(&case.bundle("build"));
            if let Some(expected) = &baseline {
                assert_eq!(&actual, expected, "{fixture}/{provider}");
            } else {
                baseline = Some(actual);
            }
            case.clear("build");
        }
    }
}

#[test]
fn genuine_empty_vector_and_array_bounds_use_only_controlled_source_trap() {
    let _guard = support::guard();
    let case = Case::new("bounds");
    let node = support::node();
    for vector in [false, true] {
        for target in targets() {
            let mut command = case.command("run", target, vector, "native", Some(&node));
            command.args(["--export", "entry", "--arg=i32:0"]).arg(format!("--arg=bool:{vector}"));
            let response = successful(&output(&mut command), &case, "run");
            for record in response["results"].as_array().expect("complete outcomes") {
                assert_eq!(
                    record["outcome"],
                    json!({"kind":"trapped", "code":"zryna.trap.bounds-v1"})
                );
            }
            let metadata = manifest(&case, "run");
            assert_eq!(metadata["invocation"]["outcomes"], response["results"]);
            case.clear("run");
        }
    }
}

#[test]
fn guide_full_source_fences_match_originals_and_execute() {
    let _guard = support::guard();
    let guide = fs::read_to_string(support::repo().join("docs/M7_GETTING_STARTED.md"))
        .expect("public generic guide must supply complete frozen source examples");
    let node = support::node();
    for (heading, fixture) in
        [("scalar-clone", "scalar"), ("module-entry", "entry-one"), ("mixed-catalogue", "mixed")]
    {
        let section = guide.split(&format!("### {heading}\n")).nth(1).expect("named guide section");
        let section = section.split("\n##").next().expect("section boundary");
        let fences = section
            .split("```zry\n")
            .skip(1)
            .map(|block| block.split("```").next().expect("complete source fence"))
            .collect::<Vec<_>>();
        let frozen = pins();
        let files = frozen["fixtures"][fixture]["files"].as_array().expect("original file order");
        assert_eq!(fences.len(), files.len());
        let case = Case::new(fixture);
        for (fence, row) in fences.iter().zip(files) {
            assert_eq!(fence.as_bytes(), support::original(row));
            fs::write(
                case.root.join(&case.directory).join(row["member"].as_str().expect("file member")),
                fence,
            )
            .expect("the exact published example is compiler input");
        }
        assert!(section.contains("--profile bounded-generics-v1"));
        let documented = section
            .lines()
            .find(|line| line.starts_with("zryna run "))
            .expect("one documented public run command");
        let target = documented
            .split("--target ")
            .nth(1)
            .expect("documented target")
            .split_whitespace()
            .next()
            .expect("target value");
        assert!(documented.contains("--export entry") && documented.contains("--node \"$NODE\""));
        let mut command =
            case.command("run", target, documented.contains("--profile="), "native", Some(&node));
        command.args(["--export", "entry"]);
        let response = successful(&output(&mut command), &case, "run");
        assert_eq!(
            response["results"][0]["outcome"]["value"],
            json!({"type":"i32", "value":frozen["fixtures"][fixture]["result_i32"]})
        );
    }
}

fn successful(output: &Output, case: &Case, action: &str) -> Value {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).expect("public success JSON");
    assert_eq!(value["version"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], action);
    assert_eq!(value["diagnostics"], serde_json::json!([]));
    assert_eq!(
        value["manifest"],
        format!(".zryna/out/{}.{action}/zryna-generic-manifest-v1.json", case.stem)
    );
    value
}

fn targets() -> &'static [&'static str] {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        &["all"]
    } else {
        &["javascript", "webassembly"]
    }
}
