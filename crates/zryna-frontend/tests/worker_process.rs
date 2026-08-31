//! Cross-platform subprocess contract checks with a self-hosted fake provider.

#![forbid(unsafe_code)]

use std::{
    env,
    ffi::OsString,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process, thread,
    time::Duration,
};

use serde_json::{Value, json};
use zryna_frontend::{
    FrontendCapabilities, ProviderExpectation, ProviderExpectationV3, WorkerFailure,
    WorkerFrontend, WorkerFrontendV3, WorkerLimits, WorkerLimitsV3, WorkerSpec, WorkerSpecV3,
};
use zryna_source::{SourceFileInput, SourceMap};

const EXPECTED_LITERAL: &str = "space & echo $HOME ; %PATH% | <literal>";

fn main() {
    let arguments: Vec<OsString> = env::args_os().collect();
    if arguments.get(1).is_some_and(|argument| argument == "--fake-worker") {
        fake_worker(&arguments);
        return;
    }
    run_process_contract_tests();
}

fn run_process_contract_tests() {
    let valid = run("valid", &[], default_test_limits()).expect("valid worker must succeed");
    assert_eq!(valid.files().len(), 1);
    assert_eq!(valid.files()[0].path().as_str(), "src/main.zry");

    expect_failure("wrong-id", &[], WorkerFailure::InvalidResponse);
    expect_failure("malformed-handshake", &[], WorkerFailure::InvalidResponse);
    expect_failure("duplicate-handshake", &[], WorkerFailure::InvalidResponse);
    expect_failure("blank-handshake", &[], WorkerFailure::InvalidResponse);
    expect_failure("provider-error", &[], WorkerFailure::ProviderRejected);
    expect_failure("wrong-provider", &[], WorkerFailure::ProviderIdentity);
    expect_failure("wrong-version", &[], WorkerFailure::ProviderVersion);
    expect_failure("wrong-protocol", &[], WorkerFailure::ProviderProtocol);
    expect_failure("wrong-module-capability", &[], WorkerFailure::ProviderCapabilities);
    expect_failure("wrong-semantic-capability", &[], WorkerFailure::ProviderCapabilities);
    expect_failure("missing-analysis", &[], WorkerFailure::InvalidResponse);
    expect_failure("wrong-analysis-id", &[], WorkerFailure::InvalidResponse);
    expect_failure("malformed-analysis", &[], WorkerFailure::InvalidResponse);
    expect_failure("extra-response", &[], WorkerFailure::InvalidResponse);
    expect_failure("trailing-output", &[], WorkerFailure::InvalidResponse);
    expect_failure("crash-before-analysis", &[], WorkerFailure::InvalidResponse);
    expect_failure("nonzero-after-result", &[], WorkerFailure::ProcessExit);
    expect_failure("invalid-snapshot", &[], WorkerFailure::SnapshotVerification);

    let timeout_limits = WorkerLimits::new(
        Duration::from_secs(1),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded timeout");
    expect_failure_with_limits(
        "hang-before-handshake",
        &[],
        WorkerFailure::Timeout,
        timeout_limits,
    );
    expect_failure_with_limits("hang-after-result", &[], WorkerFailure::Timeout, timeout_limits);

    let stderr_limits =
        WorkerLimits::new(Duration::from_secs(2), zryna_frontend::MAX_WORKER_STDOUT_BYTES, 1_024)
            .expect("bounded stderr");
    expect_failure_with_limits("stderr-overflow", &[], WorkerFailure::OutputLimit, stderr_limits);
    let stdout_limits =
        WorkerLimits::new(Duration::from_secs(2), 32, 1_024).expect("bounded stdout");
    expect_failure_with_limits("valid", &[], WorkerFailure::OutputLimit, stdout_limits);

    run("literal-argument", &[EXPECTED_LITERAL], default_test_limits())
        .expect("metacharacters must remain one literal argument");
    assert_no_analysis_before_verified_handshake();
    assert_timeout_kills_and_reaps_child();
    assert_timeout_kills_descendant_tree();
    assert_cleanup_waits_for_closed_stdio_descendant();
    run("environment-isolated", &[], default_test_limits())
        .expect("worker must not inherit the caller environment");
    expect_failure("invalid-utf8", &[], WorkerFailure::InvalidResponse);
    assert_large_unread_request_times_out_and_cleans_up();
    assert_protocol_v3_process_contract();

    println!("frontend worker process contract passed");
}

fn assert_protocol_v3_process_contract() {
    let snapshot = run_v3("valid-v3").expect("exact v3 worker must succeed");
    assert_eq!(snapshot.schema_version(), 3);
    assert_eq!(snapshot.files().len(), 1);
    assert_eq!(snapshot.files()[0].path().as_str(), "src/main.zry");

    for (mode, expected) in [
        ("v3-wrong-protocol", WorkerFailure::ProviderProtocol),
        ("v3-wrong-control-capability", WorkerFailure::ProviderCapabilities),
        ("v3-extra-capability", WorkerFailure::InvalidResponse),
        ("v3-missing-capability", WorkerFailure::InvalidResponse),
        ("v3-invalid-snapshot", WorkerFailure::SnapshotVerification),
    ] {
        let error = run_v3(mode).expect_err("malformed v3 worker must fail closed");
        assert_eq!(error.failure(), expected, "unexpected v3 failure for {mode}");
    }

    assert!(
        WorkerLimitsV3::new(
            Duration::from_secs(2),
            zryna_frontend::MAX_WORKER_STDOUT_BYTES_V3 + 1,
            zryna_frontend::MAX_WORKER_STDERR_BYTES,
        )
        .is_err()
    );
}

fn run_v3(
    mode: &str,
) -> Result<zryna_frontend::syntax_v3::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    let executable = env::current_exe().expect("test executable path");
    let current_dir = env::current_dir().expect("test working directory");
    let expected =
        ProviderExpectationV3::new("typescript-6", "6.0.3").expect("trusted v3 expectation");
    let spec = WorkerSpecV3::new(
        executable,
        vec![OsString::from("--fake-worker"), OsString::from(mode)],
        current_dir,
        expected,
        WorkerLimitsV3::new(
            Duration::from_secs(2),
            zryna_frontend::MAX_WORKER_STDOUT_BYTES_V3,
            zryna_frontend::MAX_WORKER_STDERR_BYTES,
        )
        .expect("bounded v3 limits"),
    )
    .expect("absolute direct v3 command");
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: String::new(),
    }])
    .expect("bounded source map");
    WorkerFrontendV3::new(spec).analyze_verified_v3(&sources)
}

fn expect_failure(mode: &str, arguments: &[&str], expected: WorkerFailure) {
    expect_failure_with_limits(mode, arguments, expected, default_test_limits());
}

fn expect_failure_with_limits(
    mode: &str,
    arguments: &[&str],
    expected: WorkerFailure,
    limits: WorkerLimits,
) {
    let error = run(mode, arguments, limits).expect_err("worker mode must fail closed");
    assert_eq!(error.failure(), expected, "unexpected failure for {mode}");
}

fn run(
    mode: &str,
    arguments: &[&str],
    limits: WorkerLimits,
) -> Result<zryna_frontend::syntax_v2::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    run_with_text(mode, arguments, limits, String::new())
}

fn run_with_text(
    mode: &str,
    arguments: &[&str],
    limits: WorkerLimits,
    text: String,
) -> Result<zryna_frontend::syntax_v2::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    let executable = env::current_exe().expect("test executable path");
    let current_dir = env::current_dir().expect("test working directory");
    let mut worker_arguments = vec![OsString::from("--fake-worker"), OsString::from(mode)];
    worker_arguments.extend(arguments.iter().map(OsString::from));
    let expected = ProviderExpectation::new(
        "typescript-6",
        "6.0.3",
        2,
        FrontendCapabilities { module_resolution: false, semantic_diagnostics: false },
    )
    .expect("trusted expectation");
    let spec = WorkerSpec::new(executable, worker_arguments, current_dir, expected, limits)
        .expect("absolute direct command");
    let sources = SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text }])
        .expect("bounded source map");
    WorkerFrontend::new(spec).analyze_verified(&sources)
}

fn default_test_limits() -> WorkerLimits {
    WorkerLimits::new(
        Duration::from_secs(2),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded test limits")
}

fn marker_path(label: &str) -> PathBuf {
    env::temp_dir().join(format!("zryna-worker-{}-{label}.marker", process::id()))
}

fn assert_no_analysis_before_verified_handshake() {
    let marker = marker_path("no-analysis-before-handshake");
    let _ = fs::remove_file(&marker);
    expect_failure(
        "wrong-provider-probe",
        &[marker.to_str().expect("UTF-8 marker path")],
        WorkerFailure::ProviderIdentity,
    );
    thread::sleep(Duration::from_millis(200));
    assert!(!marker.exists(), "analysis was written before handshake verification");
}

fn assert_timeout_kills_and_reaps_child() {
    let marker = marker_path("timeout-cleanup");
    let _ = fs::remove_file(&marker);
    let limits = WorkerLimits::new(
        Duration::from_secs(1),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded timeout");
    expect_failure_with_limits(
        "delayed-marker",
        &[marker.to_str().expect("UTF-8 marker path")],
        WorkerFailure::Timeout,
        limits,
    );
    thread::sleep(Duration::from_secs(1));
    assert!(!marker.exists(), "timed-out direct child remained alive");
}

fn assert_timeout_kills_descendant_tree() {
    let marker = marker_path("descendant-cleanup");
    let _ = fs::remove_file(&marker);
    let limits = WorkerLimits::new(
        Duration::from_secs(1),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded timeout");
    expect_failure_with_limits(
        "descendant-holds-pipes",
        &[marker.to_str().expect("UTF-8 marker path")],
        WorkerFailure::Timeout,
        limits,
    );
    thread::sleep(Duration::from_secs(1));
    assert!(!marker.exists(), "timed-out descendant process remained alive");
}

fn assert_cleanup_waits_for_closed_stdio_descendant() {
    let marker = marker_path("closed-stdio-descendant-cleanup");
    let _ = fs::remove_file(&marker);
    let limits = WorkerLimits::new(
        Duration::from_secs(6),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded cleanup timeout");
    expect_failure_with_limits(
        "descendant-closed-stdio",
        &[marker.to_str().expect("UTF-8 marker path")],
        WorkerFailure::InvalidResponse,
        limits,
    );
    thread::sleep(Duration::from_secs(1));
    assert!(!marker.exists(), "closed-stdio descendant survived cleanup");
}

fn assert_large_unread_request_times_out_and_cleans_up() {
    let limits = WorkerLimits::new(
        Duration::from_secs(1),
        zryna_frontend::MAX_WORKER_STDOUT_BYTES,
        zryna_frontend::MAX_WORKER_STDERR_BYTES,
    )
    .expect("bounded timeout");
    let error = run_with_text("no-read-analysis", &[], limits, "x".repeat(2 * 1_024 * 1_024))
        .expect_err("blocked request writer must time out");
    assert_eq!(error.failure(), WorkerFailure::Timeout);
}

fn fake_worker(arguments: &[OsString]) {
    let mode = arguments.get(2).and_then(|value| value.to_str()).unwrap_or("invalid");
    if mode == "hang-before-handshake" {
        thread::sleep(Duration::from_mins(1));
        return;
    }
    if mode == "delayed-marker" {
        thread::sleep(Duration::from_millis(800));
        if let Some(path) = arguments.get(3) {
            fs::write(Path::new(path), b"alive").expect("write delayed marker");
        }
        return;
    }
    if mode == "descendant-marker" {
        thread::sleep(Duration::from_millis(800));
        if let Some(path) = arguments.get(3) {
            fs::write(Path::new(path), b"alive").expect("write descendant marker");
        }
        thread::sleep(Duration::from_mins(1));
        return;
    }

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut line = String::new();
    if input.read_line(&mut line).expect("read handshake") == 0 {
        process::exit(10);
    }

    write_handshake_for_mode(&mut output, mode);

    if mode == "no-read-analysis" {
        thread::sleep(Duration::from_mins(1));
        return;
    }

    if is_rejected_handshake_mode(mode) {
        line.clear();
        if input.read_line(&mut line).unwrap_or(0) > 0
            && mode == "wrong-provider-probe"
            && let Some(path) = arguments.get(3)
        {
            fs::write(Path::new(path), b"analysis received").expect("write analysis probe");
        }
        thread::sleep(Duration::from_mins(1));
        return;
    }

    line.clear();
    if input.read_line(&mut line).expect("read analysis") == 0 {
        process::exit(11);
    }
    write_analysis_for_mode(&mut output, arguments, mode, &line);
}

fn write_handshake_for_mode(output: &mut impl Write, mode: &str) {
    match mode {
        "valid-v3" | "v3-invalid-snapshot" => write_handshake_v3(output, 3, true, None),
        "v3-wrong-protocol" => write_handshake_v3(output, 2, true, None),
        "v3-wrong-control-capability" => write_handshake_v3(output, 3, false, None),
        "v3-extra-capability" => write_handshake_v3(output, 3, true, Some(("extra", true))),
        "v3-missing-capability" => write_line(
            output,
            r#"{"id":1,"result":{"provider":"typescript-6","provider_version":"6.0.3","protocol_version":3,"capabilities":{"module_resolution":false,"semantic_diagnostics":false}}}"#,
        ),
        "malformed-handshake" => write_line(output, "{"),
        "duplicate-handshake" => write_line(
            output,
            r#"{"id":1,"id":1,"result":{"provider":"typescript-6","provider_version":"6.0.3","protocol_version":2,"capabilities":{"module_resolution":false,"semantic_diagnostics":false}}}"#,
        ),
        "blank-handshake" => write_line(output, ""),
        "wrong-id" => write_handshake(output, 99, "typescript-6", "6.0.3", 2, false, false),
        "provider-error" => {
            write_line(output, r#"{"id":1,"error":{"code":"F","message":"rejected"}}"#);
        }
        "wrong-provider" | "wrong-provider-probe" => {
            write_handshake(output, 1, "other", "6.0.3", 2, false, false);
        }
        "wrong-version" => {
            write_handshake(output, 1, "typescript-6", "6.0.2", 2, false, false);
        }
        "wrong-protocol" => {
            write_handshake(output, 1, "typescript-6", "6.0.3", 1, false, false);
        }
        "wrong-module-capability" => {
            write_handshake(output, 1, "typescript-6", "6.0.3", 2, true, false);
        }
        "wrong-semantic-capability" => {
            write_handshake(output, 1, "typescript-6", "6.0.3", 2, false, true);
        }
        _ => write_handshake(output, 1, "typescript-6", "6.0.3", 2, false, false),
    }
}

fn is_rejected_handshake_mode(mode: &str) -> bool {
    matches!(
        mode,
        "malformed-handshake"
            | "duplicate-handshake"
            | "blank-handshake"
            | "wrong-id"
            | "provider-error"
            | "wrong-provider"
            | "wrong-provider-probe"
            | "wrong-version"
            | "wrong-protocol"
            | "wrong-module-capability"
            | "wrong-semantic-capability"
            | "v3-wrong-protocol"
            | "v3-wrong-control-capability"
            | "v3-extra-capability"
            | "v3-missing-capability"
    )
}

fn write_analysis_for_mode(
    output: &mut impl Write,
    arguments: &[OsString],
    mode: &str,
    line: &str,
) {
    if matches!(mode, "descendant-holds-pipes" | "descendant-closed-stdio") {
        let executable = env::current_exe().expect("descendant executable");
        let marker = arguments.get(3).expect("descendant marker path");
        if mode == "descendant-closed-stdio" {
            spawn_closed_stdio_descendant(&executable, marker);
            process::exit(0);
        }
        let mut command = process::Command::new(executable);
        command.arg("--fake-worker").arg("descendant-marker").arg(marker);
        let mut descendant = command.spawn().expect("spawn descendant");
        thread::spawn(move || {
            let _ = descendant.wait();
        });
        thread::sleep(Duration::from_mins(1));
        return;
    }
    if mode == "literal-argument"
        && arguments.get(3).and_then(|value| value.to_str()) != Some(EXPECTED_LITERAL)
    {
        process::exit(12);
    }
    if mode == "missing-analysis" {
        return;
    }
    if mode == "crash-before-analysis" {
        process::exit(13);
    }
    if mode == "malformed-analysis" {
        write_line(output, "not-json");
        return;
    }
    if mode == "invalid-utf8" {
        output.write_all(&[0xff, b'\n']).expect("write invalid UTF-8");
        output.flush().expect("flush invalid UTF-8");
        return;
    }
    if mode == "environment-isolated" && env::var_os("PATH").is_some() {
        process::exit(15);
    }

    let request: Value = serde_json::from_str(line).expect("valid analyze request");
    let files: Vec<Value> = request["params"]["files"]
        .as_array()
        .expect("files array")
        .iter()
        .enumerate()
        .map(|(index, file)| {
            json!({
                "id": index,
                "path": file["path"],
                "functions": []
            })
        })
        .collect();
    let is_v3 = mode.starts_with("v3-") || mode == "valid-v3";
    if is_v3 {
        assert_eq!(request["params"]["schema_version"], 3);
    }
    let snapshot = if mode == "invalid-snapshot" {
        json!({"schema_version": 2, "files": [{"id": 0, "path": "wrong.zry", "functions": []}], "diagnostics": []})
    } else if mode == "v3-invalid-snapshot" {
        json!({"schema_version": 3, "files": [{"id": 0, "path": "wrong.zry", "imports": [], "functions": []}], "diagnostics": []})
    } else if is_v3 {
        let files = request["params"]["files"]
            .as_array()
            .expect("files array")
            .iter()
            .enumerate()
            .map(|(index, file)| {
                json!({
                    "id": index,
                    "path": file["path"],
                    "imports": [],
                    "functions": []
                })
            })
            .collect::<Vec<_>>();
        json!({"schema_version": 3, "files": files, "diagnostics": []})
    } else {
        json!({"schema_version": 2, "files": files, "diagnostics": []})
    };
    let response_id = if mode == "wrong-analysis-id" { 77 } else { 2 };
    let response = json!({"id": response_id, "result": snapshot}).to_string();

    if mode == "stderr-overflow" {
        std::io::stderr().write_all(&vec![b'e'; 1_025]).expect("write stderr");
    }
    write_line(output, &response);
    match mode {
        "extra-response" => write_line(output, &response),
        "trailing-output" => {
            output.write_all(b"trailing").expect("write trailing output");
            output.flush().expect("flush trailing output");
        }
        "nonzero-after-result" => process::exit(14),
        "hang-after-result" => thread::sleep(Duration::from_mins(1)),
        _ => {}
    }
}

#[cfg(windows)]
fn spawn_closed_stdio_descendant(executable: &Path, marker: &OsString) {
    let mut command = windows_spawn::Command::new(executable);
    command
        .arg("--fake-worker")
        .arg("descendant-marker")
        .arg(marker)
        .stdin(windows_spawn::Stdio::null())
        .stdout(windows_spawn::Stdio::null())
        .stderr(windows_spawn::Stdio::null());
    let mut descendant = command.spawn().expect("spawn closed-stdio descendant");
    thread::spawn(move || {
        let _ = descendant.wait();
    });
}

#[cfg(not(windows))]
fn spawn_closed_stdio_descendant(executable: &Path, marker: &OsString) {
    let mut command = process::Command::new(executable);
    command
        .arg("--fake-worker")
        .arg("descendant-marker")
        .arg(marker)
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null());
    let mut descendant = command.spawn().expect("spawn closed-stdio descendant");
    thread::spawn(move || {
        let _ = descendant.wait();
    });
}

fn write_handshake(
    output: &mut impl Write,
    id: u32,
    provider: &str,
    version: &str,
    protocol: u32,
    module_resolution: bool,
    semantic_diagnostics: bool,
) {
    write_line(
        output,
        &json!({
            "id": id,
            "result": {
                "provider": provider,
                "provider_version": version,
                "protocol_version": protocol,
                "capabilities": {
                    "module_resolution": module_resolution,
                    "semantic_diagnostics": semantic_diagnostics
                }
            }
        })
        .to_string(),
    );
}

fn write_handshake_v3(
    output: &mut impl Write,
    protocol: u32,
    control_flow_v1: bool,
    extra: Option<(&str, bool)>,
) {
    let mut capabilities = json!({
        "module_resolution": false,
        "semantic_diagnostics": false,
        "control_flow_v1": control_flow_v1
    });
    if let Some((name, value)) = extra {
        capabilities
            .as_object_mut()
            .expect("capabilities object")
            .insert(name.to_owned(), Value::Bool(value));
    }
    write_line(
        output,
        &json!({
            "id": 1,
            "result": {
                "provider": "typescript-6",
                "provider_version": "6.0.3",
                "protocol_version": protocol,
                "capabilities": capabilities
            }
        })
        .to_string(),
    );
}

fn write_line(output: &mut impl Write, line: &str) {
    output.write_all(line.as_bytes()).expect("write fake response");
    output.write_all(b"\n").expect("terminate fake response");
    output.flush().expect("flush fake response");
}
