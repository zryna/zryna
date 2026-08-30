//! Executable-level acceptance tests for the public M1 CLI contract.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::process::Stdio;

use serde_json::{Value, json};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct WorkspaceCase {
    root: PathBuf,
    source_relative: String,
    owned_paths: Vec<PathBuf>,
    unique: String,
}

impl WorkspaceCase {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository workspace root must resolve");
        let sequence = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        let unique = format!("zryna_cli_{}_{}", std::process::id(), sequence);
        let source_relative = format!(".zryna/cache/{unique}/main.zry");
        let source = root.join(&source_relative);
        fs::create_dir_all(source.parent().expect("source parent must exist"))
            .expect("test source directory must be created");
        fs::write(&source, include_str!("../../../examples/universal/add.zry"))
            .expect("test source must be written");
        Self { root, source_relative, owned_paths: Vec::new(), unique }
    }

    fn stem(&mut self, label: &str, command: &str) -> String {
        let stem = format!("{}_{}", self.unique, label);
        self.owned_paths.push(self.root.join(".zryna/out").join(format!("{stem}.{command}")));
        stem
    }

    fn missing_entrypoint(&self) -> String {
        format!(".zryna/cache/{}/missing.zry", self.unique)
    }

    fn bundle(&self, stem: &str, command: &str) -> PathBuf {
        self.root.join(".zryna/out").join(format!("{stem}.{command}"))
    }
}

impl Drop for WorkspaceCase {
    fn drop(&mut self) {
        for path in &self.owned_paths {
            let _ = fs::remove_dir_all(path);
        }
        let _ = fs::remove_dir_all(self.root.join(".zryna/cache").join(&self.unique));
    }
}

struct InvalidWorkspace {
    root: PathBuf,
}

impl InvalidWorkspace {
    fn new() -> Self {
        let sequence = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        let root =
            env::temp_dir().join(format!("zryna-cli-invalid-{}-{sequence}", std::process::id()));
        fs::create_dir_all(root.join("src")).expect("invalid fixture directory must be created");
        fs::write(root.join("src/main.zry"), "export function value(): i32 { return 1; }")
            .expect("invalid fixture source must be written");
        Self { root }
    }
}

impl Drop for InvalidWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn zryna() -> Command {
    Command::new(env!("CARGO_BIN_EXE_zryna"))
}

fn node_executable() -> PathBuf {
    let configured = ["ZRYNA_TEST_NODE", "NODE"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .chain({
            let executable = if cfg!(windows) { "node.exe" } else { "node" };
            env::var_os("PATH")
                .into_iter()
                .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
                .map(move |directory| directory.join(executable))
        })
        .find_map(|candidate| candidate.is_file().then(|| candidate.canonicalize().ok()).flatten())
        .expect("Node.js 22.22.1 must be available to CLI integration tests");
    let output = Command::new(&configured)
        .arg("--version")
        .output()
        .expect("Node.js version probe must start");
    assert!(output.status.success(), "Node.js version probe must succeed");
    assert_eq!(output.stdout, b"v22.22.1\n");
    assert!(output.stderr.is_empty());
    configured
}

fn dummy_absolute_node() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\zryna-test-missing-node.exe")
    } else {
        PathBuf::from("/zryna-test-missing-node")
    }
}

fn command_output(case: &WorkspaceCase, arguments: &[&str]) -> Output {
    zryna()
        .args(arguments)
        .arg("--root")
        .arg(&case.root)
        .arg("--node")
        .arg(node_executable())
        .output()
        .expect("zryna CLI must start")
}

#[cfg(unix)]
fn compile_test_node(case: &WorkspaceCase, label: &str, target_body: &str) -> PathBuf {
    let wrapper =
        case.root.join(".zryna/cache").join(&case.unique).join(format!("node-wrapper-{label}"));
    let wrapper_source = wrapper.with_extension("rs");
    let real_node = node_executable();
    let real_node_literal = serde_json::to_string(real_node.to_string_lossy().as_ref())
        .expect("Node path must serialize as a source literal");
    fs::write(
        &wrapper_source,
        format!(
            r#"fn main() {{
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.first().is_some_and(|value| value == "--version") {{
        print!("v22.22.1\n");
        return;
    }}
    if arguments.first().and_then(|value| std::path::Path::new(value).file_name()).is_some_and(|value| value == "worker.mjs") {{
        let status = std::process::Command::new({real_node_literal}).args(&arguments).status().expect("real Node must start");
        std::process::exit(status.code().unwrap_or(1));
    }}
    {target_body}
}}
"#
        ),
    )
    .expect("runtime wrapper source must be written");
    let compile = Command::new("rustc")
        .args(["--edition=2024", "-o"])
        .arg(&wrapper)
        .arg(&wrapper_source)
        .output()
        .expect("runtime wrapper compiler must start");
    assert_success(&compile);
    wrapper
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "successful command wrote stderr");
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("JSON file must be readable"))
        .expect("JSON file must decode")
}

#[test]
fn help_and_required_target_and_node_are_stable() {
    for arguments in
        [["--help"].as_slice(), ["build", "--help"].as_slice(), ["run", "--help"].as_slice()]
    {
        let output = zryna().args(arguments).output().expect("help command must start");
        assert_eq!(output.status.code(), Some(0), "help must succeed: {arguments:?}");
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).expect("help must be UTF-8");
        assert!(stdout.contains("Usage:"));
    }

    let missing_target = zryna()
        .args(["build", "src/main.zry", "--node"])
        .arg(dummy_absolute_node())
        .output()
        .expect("missing-target command must start");
    assert_eq!(missing_target.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_target.stderr).contains("--target"));

    let missing_node = zryna()
        .args(["build", "src/main.zry", "--target", "javascript"])
        .output()
        .expect("missing-node command must start");
    assert_eq!(missing_node.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_node.stderr).contains("--node"));
}

#[test]
fn canonical_scalar_arguments_are_enforced_by_the_executable_parser() {
    for rejected in ["i32:+1", "i32:01", "i32:-0", "i32: 1", "bool:true"] {
        let output = zryna()
            .args([
                "run",
                "src/main.zry",
                "--target",
                "javascript",
                "--export",
                "add",
                "--arg",
                rejected,
                "--node",
            ])
            .arg(dummy_absolute_node())
            .output()
            .expect("invalid-argument command must start");
        assert_eq!(output.status.code(), Some(2), "{rejected} must be rejected");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("canonical"));
    }
}

#[test]
fn architecture_failure_precedes_output_creation() {
    let workspace = InvalidWorkspace::new();
    let output = zryna()
        .args([
            "build",
            "src/main.zry",
            "--target",
            "javascript",
            "--name",
            "must_not_exist",
            "--root",
        ])
        .arg(&workspace.root)
        .arg("--node")
        .arg(dummy_absolute_node())
        .output()
        .expect("architecture-failure command must start");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ZRYNA-A"));
    assert!(!workspace.root.join(".zryna").exists());
}

#[test]
fn javascript_build_and_run_publish_exact_bundles() {
    let mut case = WorkspaceCase::new();
    let build_stem = case.stem("javascript_build", "build");
    let build = command_output(
        &case,
        &["build", &case.source_relative, "--target", "javascript", "--name", &build_stem],
    );
    assert_success(&build);
    let build_bundle = case.bundle(&build_stem, "build");
    let manifest_path = build_bundle.join("zryna-manifest-v1.json");
    assert_eq!(
        build.stdout,
        format!(".zryna/out/{build_stem}.build/zryna-manifest-v1.json\n").as_bytes()
    );
    assert!(build_bundle.join(format!("javascript/{build_stem}.mjs")).is_file());
    assert!(!build_bundle.join("webassembly").exists());
    assert!(!build_bundle.join("native").exists());
    let manifest = read_json(&manifest_path);
    assert_eq!(manifest["command"], "build");
    assert_eq!(manifest["targets"], json!(["javascript"]));
    assert_eq!(manifest["results"], json!([]));

    let run_stem = case.stem("javascript_run", "run");
    let run = command_output(
        &case,
        &[
            "run",
            &case.source_relative,
            "--target",
            "javascript",
            "--name",
            &run_stem,
            "--export",
            "add",
            "--arg=i32:20",
            "--arg=i32:22",
        ],
    );
    assert_success(&run);
    assert_eq!(run.stdout, b"javascript: i32 42\n");
    let run_bundle = case.bundle(&run_stem, "run");
    assert!(run_bundle.join(format!("javascript/{run_stem}.mjs")).is_file());
    let manifest = read_json(&run_bundle.join("zryna-manifest-v1.json"));
    assert_eq!(
        manifest["invocation"]["arguments"],
        json!([
            {"type": "i32", "value": 20},
            {"type": "i32", "value": 22}
        ])
    );
    assert_eq!(manifest["results"][0]["outcome"]["value"], 42);
}

#[test]
fn webassembly_run_uses_canonical_i32_boundaries() {
    let mut case = WorkspaceCase::new();
    let stem = case.stem("webassembly_run", "run");
    let output = command_output(
        &case,
        &[
            "run",
            &case.source_relative,
            "--target",
            "webassembly",
            "--name",
            &stem,
            "--export",
            "add",
            "--arg=i32:2147483647",
            "--arg=i32:1",
        ],
    );
    assert_success(&output);
    assert_eq!(output.stdout, b"webassembly: i32 -2147483648\n");
    let bundle = case.bundle(&stem, "run");
    assert!(bundle.join(format!("webassembly/{stem}.wasm")).is_file());
    let manifest = read_json(&bundle.join("zryna-manifest-v1.json"));
    assert_eq!(manifest["targets"], json!(["webassembly"]));
    assert_eq!(manifest["results"][0]["outcome"]["value"], i32::MIN);
}

#[test]
fn webassembly_native_and_all_builds_publish_the_selected_artifacts() {
    let mut case = WorkspaceCase::new();
    for (target, expected) in [
        ("webassembly", &["webassembly"][..]),
        ("native", &["native"][..]),
        ("all", &["javascript", "webassembly", "native"][..]),
    ] {
        let stem = case.stem(&format!("{target}_build"), "build");
        let output = command_output(
            &case,
            &["build", &case.source_relative, "--target", target, "--name", &stem],
        );
        assert_success(&output);
        let bundle = case.bundle(&stem, "build");
        let manifest = read_json(&bundle.join("zryna-manifest-v1.json"));
        assert_eq!(manifest["targets"], json!(expected));
        assert_eq!(
            bundle.join(format!("javascript/{stem}.mjs")).exists(),
            expected.contains(&"javascript")
        );
        assert_eq!(
            bundle.join(format!("webassembly/{stem}.wasm")).exists(),
            expected.contains(&"webassembly")
        );
        assert_eq!(bundle.join(format!("native/{stem}.o")).exists(), expected.contains(&"native"));
    }
}

#[test]
fn invalid_source_uses_source_exit_and_publishes_nothing() {
    let mut case = WorkspaceCase::new();
    fs::write(case.root.join(&case.source_relative), "export function broken(: i32")
        .expect("invalid source fixture must be written");
    let stem = case.stem("invalid_source", "build");
    let output = command_output(
        &case,
        &["build", &case.source_relative, "--target", "javascript", "--name", &stem],
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(!case.bundle(&stem, "build").exists());
}

#[cfg(unix)]
#[test]
fn target_execution_failure_removes_the_private_transaction() {
    let mut case = WorkspaceCase::new();
    let stem = case.stem("execution_failure", "run");
    let wrapper = compile_test_node(&case, "invalid-frame", "print!(\"bad\");");
    let child = zryna()
        .args([
            "run",
            &case.source_relative,
            "--target",
            "javascript",
            "--name",
            &stem,
            "--export",
            "add",
            "--arg=i32:20",
            "--arg=i32:22",
            "--root",
        ])
        .arg(&case.root)
        .arg("--node")
        .arg(&wrapper)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("execution-failure command must start");
    let transaction_prefix = format!(".zryna-transaction-{}-", child.id());
    let output = child.wait_with_output().expect("execution-failure command must finish");
    assert_eq!(
        output.status.code(),
        Some(5),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(!case.bundle(&stem, "run").exists());
    let leaked = fs::read_dir(case.root.join(".zryna/out"))
        .expect("output root must be readable")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .any(|name| name.starts_with(&transaction_prefix));
    assert!(!leaked, "failed target execution must not leak a transaction");
}

#[cfg(unix)]
#[test]
fn unconfirmed_transaction_cleanup_uses_exit_six_and_no_bundle() {
    let mut case = WorkspaceCase::new();
    let stem = case.stem("cleanup_failure", "run");
    let wrapper = compile_test_node(
        &case,
        "timeout",
        "std::thread::sleep(std::time::Duration::from_secs(30));",
    );
    let mut child = zryna()
        .args([
            "run",
            &case.source_relative,
            "--target",
            "javascript",
            "--name",
            &stem,
            "--export",
            "add",
            "--arg=i32:20",
            "--arg=i32:22",
            "--root",
        ])
        .arg(&case.root)
        .arg("--node")
        .arg(&wrapper)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cleanup-failure command must start");
    let transaction_prefix = format!(".zryna-transaction-{}-", child.id());
    let output_root = case.root.join(".zryna/out");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let stage = loop {
        let found = fs::read_dir(&output_root)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .find(|entry| entry.file_name().to_string_lossy().starts_with(&transaction_prefix))
            .map(|entry| entry.path());
        if let Some(stage) = found {
            break stage;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            panic!("private transaction did not appear before the test deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let displaced = output_root.join(format!("{}.displaced", case.unique));
    fs::rename(&stage, &displaced).expect("transaction stage must be displaced");
    fs::create_dir(&stage).expect("replacement transaction directory must be installed");

    let output = child.wait_with_output().expect("cleanup-failure command must finish");
    assert_eq!(
        output.status.code(),
        Some(6),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(!case.bundle(&stem, "run").exists());

    fs::remove_dir(&stage).expect("replacement transaction directory must be removed");
    fs::remove_dir_all(&displaced).expect("displaced transaction must be removed");
}

#[cfg(windows)]
#[test]
fn windows_native_and_all_runs_are_rejected_without_a_bundle() {
    let mut case = WorkspaceCase::new();
    for target in ["native", "all"] {
        let stem = case.stem(&format!("{target}_unsupported_run"), "run");
        let output = command_output(
            &case,
            &[
                "run",
                &case.source_relative,
                "--target",
                target,
                "--name",
                &stem,
                "--export",
                "add",
                "--arg=i32:20",
                "--arg=i32:22",
            ],
        );
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        assert!(!case.bundle(&stem, "run").exists());
    }
}

#[test]
fn create_only_collision_preserves_the_complete_existing_bundle() {
    let mut case = WorkspaceCase::new();
    let stem = case.stem("collision", "build");
    let arguments =
        ["build", case.source_relative.as_str(), "--target", "javascript", "--name", stem.as_str()];
    let first = command_output(&case, &arguments);
    assert_success(&first);
    let bundle = case.bundle(&stem, "build");
    let manifest_path = bundle.join("zryna-manifest-v1.json");
    let artifact_path = bundle.join(format!("javascript/{stem}.mjs"));
    let original_manifest = fs::read(&manifest_path).expect("manifest must exist");
    let original_artifact = fs::read(&artifact_path).expect("artifact must exist");
    let unrelated = case.root.join(".zryna/out").join(format!("{}.sentinel", case.unique));
    fs::write(&unrelated, b"unrelated").expect("unrelated sentinel must be written");

    let second = command_output(&case, &arguments);
    assert_eq!(second.status.code(), Some(4));
    assert!(second.stdout.is_empty());
    assert!(String::from_utf8_lossy(&second.stderr).contains("ZRYNA-C1009"));
    assert_eq!(fs::read(manifest_path).expect("manifest must remain"), original_manifest);
    assert_eq!(fs::read(artifact_path).expect("artifact must remain"), original_artifact);
    assert_eq!(fs::read(&unrelated).expect("sentinel must remain"), b"unrelated");
    fs::remove_file(unrelated).expect("owned sentinel must be removed");
}

#[test]
fn json_failure_is_one_versioned_document_with_no_advertised_bundle() {
    let mut case = WorkspaceCase::new();
    let stem = case.stem("json_failure", "build");
    let missing = case.missing_entrypoint();
    let output = command_output(
        &case,
        &["build", &missing, "--target", "javascript", "--name", &stem, "--json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be one JSON document");
    let keys = response
        .as_object()
        .expect("response must be an object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from(["command", "diagnostics", "manifest", "ok", "results", "version"])
    );
    assert_eq!(response["version"], 1);
    assert_eq!(response["ok"], false);
    assert_eq!(response["command"], "build");
    assert!(response["manifest"].is_null());
    assert_eq!(response["results"], json!([]));
    assert_eq!(response["diagnostics"][0]["code"], "ZRYNA-C1003");
    assert!(!case.bundle(&stem, "build").exists());
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
fn native_and_all_runs_publish_and_report_ordered_results() {
    let mut case = WorkspaceCase::new();
    let native_stem = case.stem("native_run", "run");
    let native = command_output(
        &case,
        &[
            "run",
            &case.source_relative,
            "--target",
            "native",
            "--name",
            &native_stem,
            "--export",
            "add",
            "--arg=i32:20",
            "--arg=i32:22",
        ],
    );
    assert_success(&native);
    assert_eq!(native.stdout, b"native: i32 42\n");
    assert!(case.bundle(&native_stem, "run").join(format!("native/{native_stem}.elf")).is_file());

    let all_stem = case.stem("all_run", "run");
    let all = command_output(
        &case,
        &[
            "run",
            &case.source_relative,
            "--target",
            "all",
            "--name",
            &all_stem,
            "--export",
            "add",
            "--arg=i32:2147483647",
            "--arg=i32:1",
        ],
    );
    assert_success(&all);
    assert_eq!(
        all.stdout,
        b"javascript: i32 -2147483648\nwebassembly: i32 -2147483648\nnative: i32 -2147483648\n"
    );
    let bundle = case.bundle(&all_stem, "run");
    assert!(bundle.join(format!("javascript/{all_stem}.mjs")).is_file());
    assert!(bundle.join(format!("webassembly/{all_stem}.wasm")).is_file());
    assert!(bundle.join(format!("native/{all_stem}.elf")).is_file());
    let manifest = read_json(&bundle.join("zryna-manifest-v1.json"));
    assert_eq!(manifest["targets"], json!(["javascript", "webassembly", "native"]));
    assert_eq!(manifest["results"].as_array().map(Vec::len), Some(3));
}
