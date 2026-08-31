//! Fixed-oracle executable conformance for the explicit M2 profile.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{Value, json};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct ConformanceWorkspace {
    root: PathBuf,
    unique: String,
    owned_bundles: Vec<PathBuf>,
}

impl ConformanceWorkspace {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository workspace root must resolve");
        let sequence = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        Self {
            root,
            unique: format!("m2_conformance_{}_{}", std::process::id(), sequence),
            owned_bundles: Vec::new(),
        }
    }

    fn stem(&mut self, label: &str, command: &str) -> String {
        let stem = format!("{}_{}", self.unique, label);
        self.owned_bundles.push(self.root.join(".zryna/out").join(format!("{stem}.{command}")));
        stem
    }

    fn bundle(&self, stem: &str, command: &str) -> PathBuf {
        self.root.join(".zryna/out").join(format!("{stem}.{command}"))
    }
}

impl Drop for ConformanceWorkspace {
    fn drop(&mut self) {
        for bundle in &self.owned_bundles {
            let _ = fs::remove_dir_all(bundle);
        }
    }
}

fn registry() -> Value {
    serde_json::from_str(include_str!("../../../tests/m2-conformance-v1.json"))
        .expect("M2 executable registry must be strict JSON")
}

fn zryna() -> Command {
    Command::new(env!("CARGO_BIN_EXE_zryna"))
}

fn node_executable() -> PathBuf {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    let node = ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .chain(
            env::var_os("PATH")
                .into_iter()
                .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
                .map(move |directory| directory.join(executable)),
        )
        .find(|path| path.is_file())
        .expect("Node.js must be installed")
        .canonicalize()
        .expect("Node.js executable must canonicalize");
    let version = Command::new(&node).arg("--version").output().expect("Node probe must start");
    assert!(version.status.success());
    assert!(matches!(version.stdout.as_slice(), b"v22.22.1\n" | b"v22.22.1\r\n"));
    assert!(version.stderr.is_empty());
    node
}

fn run_command(workspace: &ConformanceWorkspace, arguments: &[String]) -> Output {
    zryna()
        .args(arguments)
        .arg("--root")
        .arg(&workspace.root)
        .arg("--node")
        .arg(node_executable())
        .output()
        .expect("zryna CLI must start")
}

fn assert_success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "command failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("successful CLI response must be JSON")
}

fn typed_argument(argument: &Value) -> String {
    let ty = argument["type"].as_str().expect("argument type");
    let value = if ty == "bool" {
        argument["value"].as_bool().expect("bool argument").to_string()
    } else {
        argument["value"].as_i64().expect("i32 argument").to_string()
    };
    format!("--arg={ty}:{value}")
}

fn run_valid_case(
    workspace: &mut ConformanceWorkspace,
    fixture: &Value,
    target: &str,
) -> (Value, PathBuf) {
    let id = fixture["id"].as_str().expect("valid case id");
    let stem = workspace.stem(&format!("valid_{id}_{target}"), "run");
    let mut arguments = vec![
        "run".to_owned(),
        "tests/m2-fixtures/valid/main.zry".to_owned(),
        "--profile".to_owned(),
        "control-flow-v1".to_owned(),
        "--target".to_owned(),
        target.to_owned(),
        "--name".to_owned(),
        stem.clone(),
        "--export".to_owned(),
        fixture["export"].as_str().expect("valid export").to_owned(),
    ];
    arguments.extend(
        fixture["arguments"].as_array().expect("valid arguments").iter().map(typed_argument),
    );
    arguments.push("--json".to_owned());
    let response = assert_success(&run_command(workspace, &arguments));
    (response, workspace.bundle(&stem, "run"))
}

fn expected_result(target: &str, expected: &Value) -> Value {
    json!({
        "target": target,
        "outcome": {
            "kind": "returned",
            "value": expected
        }
    })
}

fn assert_graph(manifest: &Value, registry: &Value) {
    assert_eq!(manifest["profile"], "zryna-control-flow-v1");
    assert_eq!(manifest["entrypoint"], registry["graph"]["entrypoint"]);
    assert_eq!(manifest["graph_sha256"], registry["graph"]["sha256"]);
    assert_eq!(manifest["sources"], registry["graph"]["sources"]);
    assert_eq!(manifest["edges"], registry["graph"]["edges"]);
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("JSON file must be readable"))
        .expect("JSON file must decode")
}

fn bundle_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(base: &Path, current: &Path, output: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(current)
            .expect("bundle directory must be readable")
            .map(|entry| entry.expect("bundle entry must be readable"))
            .collect::<Vec<_>>();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let file_type = entry.file_type().expect("bundle type must be readable");
            assert!(!file_type.is_symlink(), "test-owned bundle must not contain links");
            let path = entry.path();
            if file_type.is_dir() {
                visit(base, &path, output);
            } else {
                assert!(file_type.is_file(), "bundle entry must be a file or directory");
                let relative = path
                    .strip_prefix(base)
                    .expect("bundle entry must remain in bundle")
                    .to_string_lossy()
                    .replace('\\', "/");
                output.insert(relative, fs::read(path).expect("bundle file must be readable"));
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
fn m2_fixed_oracle_matches_every_linux_target() {
    let registry = registry();
    let targets = ["javascript", "webassembly", "native"];
    let mut workspace = ConformanceWorkspace::new();
    for fixture in registry["validCases"].as_array().expect("valid cases") {
        let (response, bundle) = run_valid_case(&mut workspace, fixture, "all");
        let expected = &fixture["expected"];
        assert_eq!(
            response["results"],
            json!(targets.map(|target| expected_result(target, expected)))
        );
        let manifest = read_json(&bundle.join("zryna-manifest-v2.json"));
        assert_graph(&manifest, &registry);
        assert_eq!(manifest["results"].as_array().map(Vec::len), Some(3));
    }
}

#[cfg(windows)]
#[test]
fn m2_fixed_oracle_matches_every_portable_windows_target() {
    let registry = registry();
    let mut workspace = ConformanceWorkspace::new();
    for fixture in registry["validCases"].as_array().expect("valid cases") {
        for target in ["javascript", "webassembly"] {
            let (response, bundle) = run_valid_case(&mut workspace, fixture, target);
            assert_eq!(response["results"], json!([expected_result(target, &fixture["expected"])]));
            assert_graph(&read_json(&bundle.join("zryna-manifest-v2.json")), &registry);
        }
    }
}

#[test]
fn m2_invalid_programs_are_target_independent_and_publish_no_bundle() {
    let registry = registry();
    let mut workspace = ConformanceWorkspace::new();
    for fixture in registry["invalidCases"].as_array().expect("invalid cases") {
        let id = fixture["id"].as_str().expect("invalid id");
        for target in ["javascript", "webassembly", "native", "all"] {
            let stem = workspace.stem(&format!("invalid_{id}_{target}"), "build");
            let arguments = vec![
                "build".to_owned(),
                fixture["entrypoint"].as_str().expect("invalid entrypoint").to_owned(),
                "--profile".to_owned(),
                "control-flow-v1".to_owned(),
                "--target".to_owned(),
                target.to_owned(),
                "--name".to_owned(),
                stem.clone(),
                "--json".to_owned(),
            ];
            let output = run_command(&workspace, &arguments);
            let expected_exit =
                i32::try_from(fixture["exitCode"].as_i64().expect("invalid exit code"))
                    .expect("invalid exit code must fit i32");
            assert_eq!(output.status.code(), Some(expected_exit));
            assert!(output.stderr.is_empty());
            let response: Value = serde_json::from_slice(&output.stdout).expect("invalid JSON");
            assert_eq!(response["ok"], false);
            assert!(response["manifest"].is_null());
            assert_eq!(response["results"], json!([]));
            let codes = response["diagnostics"]
                .as_array()
                .expect("invalid diagnostics")
                .iter()
                .map(|diagnostic| diagnostic["code"].clone())
                .collect::<Vec<_>>();
            assert_eq!(json!(codes), fixture["diagnosticCodes"]);
            assert_eq!(response["diagnostics"], fixture["diagnostics"]);
            assert!(!workspace.bundle(&stem, "build").exists());
        }
    }
}

#[cfg(windows)]
#[test]
fn m2_windows_native_targets_remain_explicitly_unavailable() {
    let registry = registry();
    let fixture = &registry["validCases"][0];
    let mut workspace = ConformanceWorkspace::new();
    for target in ["native", "all"] {
        let id = fixture["id"].as_str().expect("valid id");
        let stem = workspace.stem(&format!("unsupported_{id}_{target}"), "run");
        let mut arguments = vec![
            "run".to_owned(),
            registry["graph"]["entrypoint"].as_str().expect("entrypoint").to_owned(),
            "--profile".to_owned(),
            "control-flow-v1".to_owned(),
            "--target".to_owned(),
            target.to_owned(),
            "--name".to_owned(),
            stem.clone(),
            "--export".to_owned(),
            fixture["export"].as_str().expect("export").to_owned(),
        ];
        arguments
            .extend(fixture["arguments"].as_array().expect("arguments").iter().map(typed_argument));
        arguments.push("--json".to_owned());
        let output = run_command(&workspace, &arguments);
        assert_eq!(output.status.code(), Some(4));
        let response: Value = serde_json::from_slice(&output.stdout).expect("unsupported JSON");
        assert_eq!(response["manifest"], Value::Null);
        assert_eq!(response["results"], json!([]));
        assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-N4002");
        assert!(!workspace.bundle(&stem, "run").exists());
    }
}

#[test]
fn m2_build_provenance_and_supported_artifacts_are_byte_deterministic() {
    let registry = registry();
    let mut workspace = ConformanceWorkspace::new();
    let targets: &[&str] =
        if cfg!(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")) {
            &["all"]
        } else {
            &["javascript", "webassembly"]
        };
    for target in targets {
        let stem = workspace.stem(&format!("determinism_{target}"), "build");
        let arguments = vec![
            "build".to_owned(),
            registry["graph"]["entrypoint"].as_str().expect("entrypoint").to_owned(),
            "--profile".to_owned(),
            "control-flow-v1".to_owned(),
            "--target".to_owned(),
            (*target).to_owned(),
            "--name".to_owned(),
            stem.clone(),
            "--json".to_owned(),
        ];
        assert_success(&run_command(&workspace, &arguments));
        let bundle = workspace.bundle(&stem, "build");
        let manifest = read_json(&bundle.join("zryna-manifest-v2.json"));
        assert_graph(&manifest, &registry);
        let expected_targets =
            if *target == "all" { registry["targetOrder"].clone() } else { json!([target]) };
        assert_eq!(manifest["targets"], expected_targets);
        let artifact_oracles =
            registry["graph"]["buildArtifacts"].as_array().expect("artifact oracles");
        for artifact in manifest["artifacts"].as_array().expect("manifest artifacts") {
            let expected = artifact_oracles
                .iter()
                .find(|candidate| candidate["target"] == artifact["target"])
                .expect("artifact target oracle");
            assert_eq!(artifact["kind"], expected["kind"]);
            assert_eq!(artifact["bytes"], expected["bytes"]);
            assert_eq!(artifact["sha256"], expected["sha256"]);
        }
        let first = bundle_bytes(&bundle);
        fs::remove_dir_all(&bundle).expect("test-owned first bundle must be removable");
        assert_success(&run_command(&workspace, &arguments));
        assert_eq!(bundle_bytes(&bundle), first);
    }
}

#[test]
fn m2_registry_case_and_fixture_ids_are_unique() {
    let registry = registry();
    for key in ["validCases", "invalidCases"] {
        let ids = registry[key]
            .as_array()
            .expect("case array")
            .iter()
            .map(|case| case["id"].as_str().expect("case id"))
            .collect::<Vec<_>>();
        assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), ids.len());
    }
    let paths = registry["fixtureFiles"]
        .as_array()
        .expect("fixture files")
        .iter()
        .map(|fixture| fixture["path"].as_str().expect("fixture path"))
        .collect::<Vec<_>>();
    assert_eq!(paths.iter().copied().collect::<BTreeSet<_>>().len(), paths.len());
}
