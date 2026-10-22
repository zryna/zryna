use std::{
    env, fs,
    process::Command,
    time::{Duration, Instant},
};

#[cfg(windows)]
use super::node_compatible_path;
use super::{
    MAX_INLINE_MODULE_BYTES, MAX_WEBASSEMBLY_INPUT_BYTES, NodeRuntimeCapability,
    is_pinned_node_version,
};
use zryna_abi::{RawHostScalar, ScalarType};

mod fixtures;
use fixtures::{RuntimeRoot, capability};

#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
#[test]
fn node_paths_remove_only_windows_verbatim_prefixes() {
    assert_eq!(
        node_compatible_path(Path::new(r"\\?\C:\workspace\adapter\src\worker.mjs")),
        PathBuf::from(r"C:\workspace\adapter\src\worker.mjs")
    );
    assert_eq!(
        node_compatible_path(Path::new(r"\\?\UNC\server\share\adapter\src\worker.mjs")),
        PathBuf::from(r"\\server\share\adapter\src\worker.mjs")
    );
    assert_eq!(
        node_compatible_path(Path::new(r"C:\workspace\adapter")),
        PathBuf::from(r"C:\workspace\adapter")
    );
}

fn assert_code(error: &zryna_diagnostics::Diagnostic, expected: &str) {
    assert_eq!(error.code(), expected);
}

#[test]
fn pinned_node_version_accepts_native_line_endings_only() {
    assert!(is_pinned_node_version(b"v22.22.1\n"));
    assert!(is_pinned_node_version(b"v22.22.1\r\n"));
    assert!(!is_pinned_node_version(b"v22.22.1"));
    assert!(!is_pinned_node_version(b"v22.22.1\nextra\n"));
    assert!(!is_pinned_node_version(b"v22.22.0\r\n"));
}

#[test]
fn webassembly_inline_inputs_fail_at_the_first_extra_byte() {
    let root = RuntimeRoot::new("webassembly-input-bounds");
    let runtime = capability(&root);
    let oversized_script = vec![b'a'; MAX_INLINE_MODULE_BYTES + 1];
    assert_code(
        &runtime
            .run_webassembly_module(&oversized_script, &[], root.path())
            .expect_err("oversized inline script must fail"),
        "ZRYNA-R3004",
    );

    let oversized_module = vec![0_u8; MAX_WEBASSEMBLY_INPUT_BYTES + 1];
    assert_code(
        &runtime
            .run_webassembly_module(b"", &oversized_module, root.path())
            .expect_err("oversized module input must fail"),
        "ZRYNA-R3004",
    );
    assert_code(
        &runtime
            .run_webassembly_module(&[0xff], &[], root.path())
            .expect_err("non-UTF-8 inline script must fail"),
        "ZRYNA-R3006",
    );
}

#[test]
fn blocked_webassembly_input_preserves_timeout_and_output_diagnostics() {
    let root = RuntimeRoot::new("blocked-webassembly-input");
    let runtime = capability(&root);
    let input = vec![0_u8; 1024 * 1024];
    let timeout = runtime
        .run_webassembly_module(b"setInterval(() => {}, 60_000);", &input, root.path())
        .expect_err("blocked input must preserve the timeout diagnostic");
    assert_code(&timeout, "ZRYNA-R3003");

    let overflow = runtime
        .run_webassembly_module(
            b"for (;;) process.stdout.write(Buffer.alloc(8192, 65));",
            &input,
            root.path(),
        )
        .expect_err("blocked input must preserve the output diagnostic");
    assert_code(&overflow, "ZRYNA-R3004");
}

#[test]
fn invalid_version_frame_stderr_and_exit_fail_closed() {
    let root = RuntimeRoot::new("invalid-results");
    let invalid_version = NodeRuntimeCapability::discover(
        &env::current_exe().expect("test executable path"),
        root.path(),
    )
    .expect_err("a non-Node executable must fail identity validation");
    assert_code(&invalid_version, "ZRYNA-R3002");

    let runtime = capability(&root);
    let short = root.harness("short-frame", "process.stdout.write(Buffer.from([1, 2, 3]));\n");
    assert_code(
        &runtime.run_module(&short, root.path()).expect_err("short frame must fail"),
        "ZRYNA-R3006",
    );

    let stderr = root.harness(
        "stderr-frame",
        "process.stdout.write(Buffer.from([1, 0, 0, 0])); process.stderr.write('unexpected');\n",
    );
    assert_code(
        &runtime.run_module(&stderr, root.path()).expect_err("stderr must fail framing"),
        "ZRYNA-R3006",
    );

    let abnormal = root.harness(
        "abnormal-exit",
        "process.stdout.write(Buffer.from([1, 0, 0, 0])); process.exit(23);\n",
    );
    assert_code(
        &runtime.run_module(&abnormal, root.path()).expect_err("abnormal exit must fail"),
        "ZRYNA-R3006",
    );
}

#[test]
fn typed_javascript_frames_use_public_exports_and_reject_noncanonical_bool() {
    let root = RuntimeRoot::new("typed-javascript");
    let runtime = capability(&root);
    let module = root.path().join("public-module.mjs");
    fs::write(
        &module,
        concat!(
            "export function Number(value) {\n",
            "  if (typeof value !== 'boolean') throw new TypeError('bool');\n",
            "  return value;\n",
            "}\n",
            "export function Object() { return -2147483648; }\n",
        ),
    )
    .expect("public module must be written");
    let module_name = serde_json::to_string(&format!(
        "./{}",
        module.file_name().expect("module file name").to_string_lossy()
    ))
    .expect("module name JSON");
    let bool_harness = root.harness(
        "bool-public-import",
        &format!(
            "import {{ Number as invoke }} from {module_name};\nconst value = invoke(true);\nif (typeof value !== 'boolean') process.exit(70);\nprocess.stdout.write(Uint8Array.of(value ? 1 : 0));\n"
        ),
    );
    assert_eq!(
        runtime
            .run_javascript_module(&bool_harness, root.path(), ScalarType::Bool)
            .expect("primitive bool frame"),
        RawHostScalar::JavaScriptBool(true)
    );

    let i32_harness = root.harness(
        "i32-public-import",
        &format!(
            "import {{ Object as invoke }} from {module_name};\nconst frame = Buffer.allocUnsafe(4);\nframe.writeInt32LE(invoke(), 0);\nprocess.stdout.write(frame);\n"
        ),
    );
    assert_eq!(
        runtime
            .run_javascript_module(&i32_harness, root.path(), ScalarType::I32)
            .expect("canonical i32 frame"),
        RawHostScalar::JavaScriptNumber(f64::from(i32::MIN))
    );

    let invalid_bool =
        root.harness("invalid-bool-frame", "process.stdout.write(Uint8Array.of(2));\n");
    assert_code(
        &runtime
            .run_javascript_module(&invalid_bool, root.path(), ScalarType::Bool)
            .expect_err("noncanonical Boolean byte must fail"),
        "ZRYNA-R3006",
    );
}

#[test]
fn live_output_overflow_fails_with_the_bounded_diagnostic() {
    let root = RuntimeRoot::new("overflow");
    let runtime = capability(&root);
    let harness =
        root.harness("overflow", "for (;;) process.stdout.write(Buffer.alloc(8192, 65));\n");

    let started = Instant::now();
    let error =
        runtime.run_module(&harness, root.path()).expect_err("unbounded target output must fail");

    assert_code(&error, "ZRYNA-R3004");
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn timeout_terminates_the_target_within_the_total_deadline() {
    let root = RuntimeRoot::new("timeout");
    let runtime = capability(&root);
    let harness = root.harness("timeout", "setInterval(() => {}, 60_000);\n");

    let started = Instant::now();
    let error =
        runtime.run_module(&harness, root.path()).expect_err("nonterminating target must time out");

    assert_code(&error, "ZRYNA-R3003");
    assert!(started.elapsed() >= Duration::from_secs(4));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn environment_is_cleared_and_metacharacter_paths_are_literal() {
    let root = RuntimeRoot::new("environment");
    let runtime = capability(&root);
    let harness = root.harness(
        "environment",
        concat!(
            "const forbidden = ['PATH', 'HOME', 'USERPROFILE', 'CARGO_HOME', 'RUSTUP_HOME'];\n",
            "if (forbidden.some((name) => process.env[name] !== undefined)) {\n",
            "  process.stderr.write('ambient environment leaked'); process.exit(31);\n",
            "}\n",
            "process.stdout.write(Buffer.from([42, 0, 0, 0]));\n",
        ),
    );

    assert_eq!(
        runtime
            .run_module(&harness, root.path())
            .expect("literal metacharacter path and cleared environment must execute"),
        [42, 0, 0, 0]
    );
}

#[test]
fn runtime_identity_replacement_is_detected_or_prevented() {
    let root = RuntimeRoot::new("identity-replacement");
    let extension = if cfg!(windows) { ".exe" } else { "" };
    let runtime_path = root.path().join(format!("private-node{extension}"));
    let runtime_source = root.path().join("private-node.rs");
    let compiled_runtime =
        env::temp_dir().join(format!("zryna-private-node-{}{}", std::process::id(), extension));
    fs::write(&runtime_source, "fn main() { print!(\"v22.22.1\\n\"); }\n")
        .expect("private runtime source must be written");
    let compiler = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = crate::process_spawn::output(
        Command::new(compiler)
            .args(["--edition=2024", "--crate-name", "zryna_private_node", "-o"])
            .arg(&compiled_runtime)
            .arg(&runtime_source),
    )
    .expect("private runtime compiler must start");
    assert!(
        output.status.success(),
        "private runtime compilation must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    fs::rename(&compiled_runtime, &runtime_path)
        .expect("compiled private runtime must move into the literal path");
    let runtime = NodeRuntimeCapability::discover(&runtime_path, root.path())
        .expect("private Node.js copy must validate");
    let retained = root.path().join(format!("retained-node{extension}"));

    match fs::rename(&runtime_path, &retained) {
        Ok(()) => {
            fs::copy(env::current_exe().expect("replacement executable path"), &runtime_path)
                .expect("replacement executable must be installed");
            assert_code(
                &runtime.revalidate().expect_err("replacement must invalidate capability"),
                "ZRYNA-R3001",
            );
        }
        Err(error) if cfg!(windows) => {
            assert!(
                error.kind() == std::io::ErrorKind::PermissionDenied
                    || matches!(error.raw_os_error(), Some(5 | 32)),
                "unexpected replacement failure: {error}"
            );
            runtime.revalidate().expect("denied replacement must preserve identity");
        }
        Err(error) => panic!("runtime replacement setup failed unexpectedly: {error}"),
    }
}
