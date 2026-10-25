//! Public failures retain original ownership diagnostics and never publish partial bundles.
use super::{
    successful,
    support::{self, Case, inventory, manifest, output, pins},
};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    process::{Command, Output},
};

#[test]
fn guide_opaque_clone_rejects_then_its_frozen_complete_correction_runs() {
    let _guard = support::guard();
    let guide = fs::read_to_string(support::repo().join("docs/M7_GETTING_STARTED.md"))
        .expect("published complete source guide");
    let section = guide
        .split("### corrected-opaque-clone\n")
        .nth(1)
        .expect("correction section")
        .split("\n##")
        .next()
        .expect("section boundary");
    let fences = section
        .split("```zry\n")
        .skip(1)
        .map(|block| block.split("```").next().expect("complete source"))
        .collect::<Vec<_>>();
    assert_eq!(fences.len(), 2);
    let frozen = pins();
    let expected = &frozen["guide"]["corrected_opaque_clone"];
    assert_eq!(format!("{:x}", Sha256::digest(fences[0].as_bytes())), expected["original_sha256"]);
    assert_eq!(
        format!("{:x}", Sha256::digest(fences[1].as_bytes())),
        expected["correction_sha256"]
    );
    let case = Case::new("unused");
    assert_eq!(
        fences[0].as_bytes(),
        fs::read(case.root.join(&case.source)).expect("exact invalid guide input")
    );
    failed(
        &output(&mut case.command("build", "all", false, "native", None)),
        &case,
        Some("ZRYNA-M7002"),
    );
    fs::write(case.root.join(&case.source), fences[1])
        .expect("only owned temporary compiler input changes");
    let node = support::node();
    let mut command = case.command("run", "javascript", false, "native", Some(&node));
    command.args(["--export", "entry"]);
    let response = successful(&output(&mut command), &case, "run");
    assert_eq!(
        response["results"][0]["outcome"]["value"],
        serde_json::json!({"type":"i32", "value":7})
    );
    manifest(&case, "run");
}

#[test]
fn unused_opaque_clone_and_template_host_selection_keep_original_spans() {
    let _guard = support::guard();
    let case = Case::new("unused");
    let response = failed(
        &output(&mut case.command("build", "all", false, "native", None)),
        &case,
        Some("ZRYNA-M7002"),
    );
    let frozen = pins();
    let expected = &frozen["fixtures"]["unused"]["diagnostic"];
    assert_eq!(response["diagnostics"][0]["primary"]["kind"], "source");
    for key in ["file", "start", "end"] {
        assert_eq!(response["diagnostics"][0]["primary"]["span"][key], expected[key]);
    }
    let case = Case::new("entry-one");
    let node = support::node();
    for name in ["identity", "identity<i32>"] {
        let mut command = case.command("run", "javascript", true, "native", Some(&node));
        command.args(["--export", name]);
        let response = failed(&output(&mut command), &case, Some("ZRYNA-M7005"));
        let frozen = pins();
        let expected = &frozen["fixtures"]["entry-one"]["template_selection"];
        for key in ["file", "start", "end"] {
            assert_eq!(response["diagnostics"][0]["primary"]["span"][key], expected[key]);
        }
    }
}

#[test]
fn invocation_validation_and_missing_explicit_node_fail_without_execution_output() {
    let _guard = support::guard();
    let case = Case::new("bounds");
    let node = support::node();
    for (name, args, code) in [
        ("entry", vec![], "ZRYNA-B2102"),
        ("entry", vec!["--arg=bool:true", "--arg=bool:true"], "ZRYNA-B2103"),
        ("missing", vec!["--arg=i32:0", "--arg=bool:true"], "ZRYNA-B2101"),
    ] {
        let mut command = case.command("run", "javascript", false, "native", Some(&node));
        command.args(["--export", name]).args(args);
        failed(&output(&mut command), &case, Some(code));
    }
    let case = Case::new("scalar");
    let mut command = case.command("run", "javascript", false, "native", None);
    command.args(["--export", "entry"]).env_remove("NODE").env_remove("ZRYNA_TEST_NODE");
    failed(&output(&mut command), &case, Some("ZRYNA-C1001"));
}

#[test]
fn unsupported_component_and_real_project_selector_fail_closed() {
    let _guard = support::guard();
    let case = Case::new("scalar");
    let node = support::node();
    failed(
        &output(&mut case.command("build", "component", false, "native", Some(&node))),
        &case,
        None,
    );
    let project = case.root.join(&case.directory).join("project");
    fs::create_dir_all(project.join("src")).expect("owned genuine source project");
    for row in pins()["project_files"].as_array().expect("genuine pinned project") {
        fs::write(
            project.join(row["member"].as_str().expect("project member")),
            support::original(row),
        )
        .expect("unchanged real package fixture");
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_zryna"));
    command
        .args([
            "build",
            "src/main.zry",
            "--profile=bounded-generics-v1",
            "--target",
            "javascript",
            "--json",
            "--root",
        ])
        .arg(&case.root)
        .arg("--project-root")
        .arg(&project)
        .args(["--name", &case.stem]);
    failed(&output(&mut command), &case, Some("ZRYNA-C2001"));
    assert!(!project.join(".zryna/out").exists());
}

#[test]
fn existing_bundle_and_file_collisions_preserve_every_byte() {
    let _guard = support::guard();
    let case = Case::new("scalar");
    for action in ["build", "run"] {
        let node = (action == "run").then(support::node);
        let make = || {
            let mut command = case.command(action, "javascript", false, "native", node.as_deref());
            if action == "run" {
                command.args(["--export", "entry"]);
            }
            command
        };
        successful(&output(&mut make()), &case, action);
        manifest(&case, action);
        let before = inventory(&case.bundle(action));
        let collision = output(&mut make());
        assert!(!collision.status.success());
        assert_eq!(inventory(&case.bundle(action)), before);
        case.clear(action);
        fs::write(case.bundle(action), b"owned-existing-destination")
            .expect("file collision control");
        assert!(!output(&mut make()).status.success());
        assert_eq!(
            fs::read(case.bundle(action)).expect("original file"),
            b"owned-existing-destination"
        );
        case.clear(action);
        successful(&output(&mut make()), &case, action);
        assert_eq!(inventory(&case.bundle(action)), before);
        case.clear(action);
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn native_run_on_other_hosts_is_an_explicit_host_rejection() {
    let _guard = support::guard();
    let case = Case::new("scalar");
    let mut command = case.command("run", "native", false, "native", None);
    command.args(["--export", "entry"]);
    failed(&output(&mut command), &case, Some("ZRYNA-N4002"));
}

fn failed(output: &Output, case: &Case, code: Option<&str>) -> Value {
    assert!(!output.status.success(), "unexpected success: {output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).expect("public rejection JSON");
    assert_eq!(value["ok"], false);
    assert!(value["manifest"].is_null());
    assert_eq!(value["results"], serde_json::json!([]));
    assert!(!value["diagnostics"].as_array().expect("diagnostics").is_empty());
    if let Some(code) = code {
        assert_eq!(value["diagnostics"][0]["code"], code);
    }
    case.no_output();
    value
}
