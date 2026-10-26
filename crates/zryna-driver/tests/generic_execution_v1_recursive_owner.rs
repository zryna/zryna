//! Unchanged deep-owner witnesses must acquire real source and execution authority first.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_recursive_owner/artifact.rs"]
mod artifact;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use zryna_frontend::{native_lexer::lex, native_parser::v5::parse_v5_candidate};
use zryna_ir::generic_v1::execution_v1;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

// Keep this test process's three deep fixtures from competing for the same child budget.
static FIXTURE_PROCESS_OWNER: Mutex<()> = Mutex::new(());

fn base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-execution-recursive-owner")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn input(name: &str) -> Vec<u8> {
    let freeze: serde_json::Value = serde_json::from_slice(
        &std::fs::read(base().join("input-identities.json")).expect("original input identities"),
    )
    .expect("original source freeze");
    let bytes = std::fs::read(base().join(name)).expect("unchanged original source input");
    let row = freeze["files"]
        .as_array()
        .expect("frozen inputs")
        .iter()
        .find(|row| row["path"] == name)
        .expect("named original input");
    assert_eq!(digest(&bytes), row["sha256"]);
    assert_eq!(u64::try_from(bytes.len()).expect("bounded input"), row["bytes"]);
    bytes
}

fn actual(case: &str) {
    let _owner = FIXTURE_PROCESS_OWNER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let text = String::from_utf8(input(&format!("literals/{case}.zry"))).expect("original UTF8");
    assert_eq!(
        digest(&input("SOURCE_EXPECTED_FREEZE.json")),
        "eb01f8297eb221c683d264fa35ab4f046f1a3ca3f7b561637090d77fd9b6ee24"
    );
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("original source map");
    let native = parse_v5_candidate(&sources, &lex(&sources).expect("original lexer"))
        .expect("native syntax candidate");
    let observations: serde_json::Value = serde_json::from_slice(
        &std::fs::read(base().join("provider-observations.json")).expect("actual TS observations"),
    )
    .expect("provider observations");
    let row = observations["records"]
        .as_array()
        .expect("actual process records")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same source process");
    let packet = std::fs::read(base().join(row["packet"].as_str().expect("packet path")))
        .expect("actual TS output");
    assert_eq!(digest(&packet), row["packet_sha256"]);
    assert_eq!(
        serde_json::to_value(&native).expect("native DTO"),
        serde_json::from_slice::<serde_json::Value>(&packet).expect("TS DTO")
    );
    for (provider, candidate) in
        [("native", native), ("typescript", decode_snapshot(&packet).expect("strict TS decode"))]
    {
        let syntax = verify_snapshot(candidate, &sources).expect("original source syntax owner");
        let entry = sources.verify_file_id(0).expect("entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
        )
        .expect("original declarations");
        let bodies = check_execution_body_types(&declarations).expect("original body types");
        let instances = semantic::discover(&bodies).expect("original semantic instances");
        let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
            .expect("original linear layouts");
        let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
            .expect("original native layouts");
        let runtime = runtime::verify_v1(
            runtime::raw_v1(&linear, &linux).expect("runtime candidate"),
            &linear,
            &linux,
        )
        .expect("same-universe runtime");
        let claim = semantic::produce_claim(&instances, &linear, &linux)
            .expect("original source execution claim");
        let wire = execution_v1::wire::encode(&claim).expect("new execution carrier");
        let program = execution_v1::verify(
            execution_v1::wire::decode(&wire).expect("execution carrier"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("independent complete source/execution/cleanup owner");
        retain(case, provider, &program, &wire);
    }
}

fn retain(case: &str, provider: &str, program: &execution_v1::VerifiedExecution<'_>, wire: &[u8]) {
    let directory = std::env::temp_dir()
        .join(format!("m416-recursive-owner-{}-{case}-{provider}", std::process::id()));
    std::fs::create_dir(&directory).expect("new observation directory");
    std::fs::write(directory.join("program.zir"), wire).expect("seal before target emission");
    let mut permissions =
        std::fs::metadata(directory.join("program.zir")).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(directory.join("program.zir"), permissions).expect("read-only wire");
    let js = zryna_backend_javascript::generic_execution_v1::emit(program);
    let wasm = zryna_backend_webassembly::generic_execution_v1::emit(program);
    let mir = zryna_native_mir::generic_execution_v1::lower(program);
    let object = mir.as_ref().ok().map(|mir| artifact::native(program, mir, &directory));
    let phases = serde_json::json!({
        "case":case,"provider":provider,"wire_sha256":digest(wire),
        "javascript":js.as_ref().err().map(|e| format!("{e:?}")),
        "webassembly":wasm.as_ref().err().map(|e| format!("{e:?}")),
        "native_mir":mir.as_ref().err().map(|e| format!("{e:?}")),
        "native_object":object.as_ref().and_then(|r| r.as_ref().err()).map(|e| format!("{e:?}")),
        "limits":"Actual original source seal and emission phases only; no runtime/deep cleanup pass."
    });
    std::fs::write(
        directory.join("phases.json"),
        serde_json::to_vec_pretty(&phases).expect("phase receipt"),
    )
    .expect("retain phase failures before assertions");
    println!("recursive source {case}/{provider}: {}", directory.display());
    if let Ok(ref js) = js {
        std::fs::write(directory.join("program.mjs"), &js.source).expect("actual JS");
    }
    if let Ok(ref wasm) = wasm {
        std::fs::write(directory.join("program.wasm"), wasm.bytes()).expect("actual Wasm");
    }
    assert!(
        js.is_ok() && wasm.is_ok() && mir.is_ok() && object.is_some_and(|r| r.is_ok()),
        "{phases}"
    );
    javascript(case, program, &directory);
    webassembly(case, program, &directory);
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("unchanged wire"), wire);
}

fn webassembly(case: &str, program: &execution_v1::VerifiedExecution<'_>, directory: &Path) {
    let observed = zryna_backend_webassembly::generic_execution_v1::emit_observed(program)
        .expect("separate same-seal private Wasm observation artifact");
    std::fs::write(directory.join("program.observed.wasm"), observed.as_bytes())
        .expect("actual observed Wasm before invocation");
    for depth in [0, 1, 8192, 65536] {
        let mut command = std::process::Command::new("node");
        command
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/generic_execution_v1_recursive_owner/wasm.mjs"),
            )
            .arg(directory)
            .arg(case)
            .arg(depth.to_string());
        let output = run_child(command, case, "webassembly", Some(depth), directory);
        std::fs::write(directory.join(format!("wasm-{depth}.stdout.json")), &output.stdout)
            .expect("actual ordinary Wasm observations");
        std::fs::write(directory.join(format!("wasm-{depth}.stderr.txt")), &output.stderr)
            .expect("actual Wasm errors before assertions");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty());
    }
}

fn javascript(case: &str, program: &execution_v1::VerifiedExecution<'_>, directory: &Path) {
    let observed = zryna_backend_javascript::generic_execution_v1::emit_observed(program)
        .expect("same-seal private observation artifact");
    std::fs::write(directory.join("program.observed.mjs"), observed.as_source())
        .expect("separate private observation artifact");
    std::fs::write(
        directory.join("SOURCE_EXPECTED_FREEZE.json"),
        input("SOURCE_EXPECTED_FREEZE.json"),
    )
    .expect("unchanged pre-execution source oracle");
    let depths: &[Option<u32>] =
        if case == "clone" { &[Some(0), Some(1), Some(8192), Some(65536)] } else { &[None] };
    for depth in depths {
        let mut command = std::process::Command::new("node");
        command
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/generic_execution_v1_recursive_owner/release.mjs"),
            )
            .arg(directory)
            .arg(case);
        let label = if let Some(depth) = depth {
            command.arg(depth.to_string());
            format!("release-{depth}")
        } else {
            "release".to_owned()
        };
        let output = run_child(command, case, "javascript", *depth, directory);
        std::fs::write(directory.join(format!("{label}.stdout.json")), &output.stdout)
            .expect("actual JS observations");
        std::fs::write(directory.join(format!("{label}.stderr.txt")), &output.stderr)
            .expect("actual JS errors");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty());
    }
}

fn run_child(
    command: std::process::Command,
    case: &str,
    phase: &str,
    depth: Option<u32>,
    directory: &Path,
) -> std::process::Output {
    eprintln!(
        "recursive owner child start case={case} phase={phase} depth={depth:?} artifact={}",
        directory.display()
    );
    let started = std::time::Instant::now();
    let output = process::run(command, 20);
    eprintln!(
        "recursive owner child complete case={case} phase={phase} depth={depth:?} elapsed_ms={} stdout_bytes={} stderr_bytes={} status={}",
        started.elapsed().as_millis(),
        output.stdout.len(),
        output.stderr.len(),
        output.status
    );
    output
}

#[test]
fn recursive_drop_original_source_and_same_seal() {
    actual("drop-only");
}
#[test]
fn recursive_clone_original_source_and_same_seal() {
    actual("clone");
}
#[test]
fn recursive_controlled_cleanup_original_source_and_same_seal() {
    actual("controlled-cleanup");
}
