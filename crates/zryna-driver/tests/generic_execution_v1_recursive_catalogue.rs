//! A genuine recursive owner forces the shared traversal kernel across the whole Clone catalogue.
#![cfg(unix)] // This Node harness uses the existing Unix process-group deadline helper.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_recursive_owner/native_mixed.rs"]
mod native_mixed;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use zryna_frontend::{native_lexer::lex, native_parser::v5::parse_v5_candidate};
use zryna_ir::generic_v1::execution_v1::{self, CloneNodeKind, VerifiedExecution};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

fn base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-recursive-owner/mixed")
}
fn pinned(name: &str, expected: &str) -> Vec<u8> {
    let bytes = std::fs::read(base().join(name)).expect("frozen genuine source material");
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
    bytes
}

#[test]
fn actual_recursive_catalogue_has_all_clone_kinds_and_controlled_failure_reentry() {
    let text = pinned(
        "mixed-catalogue.zry",
        "5d28cef948a289b7e808b5efe8a94c4c4d7f73ef5d47896195e1e2fc6077f8b6",
    );
    pinned(
        "PRE_EXECUTION_ORACLE.json",
        "bc6fdcf4d69acbd15cd5585b22bdd1a1d193be794c73177e03268d59902f4266",
    );
    let packet = pinned(
        "mixed.typescript.json",
        "918cd89ca415c91acbf589cb6a52aacbd113facdb2e24febd48a4ffe2fbea9b5",
    );
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("genuine source UTF8"),
    }])
    .expect("original source map");
    let native = parse_v5_candidate(&sources, &lex(&sources).expect("native lex"))
        .expect("actual native candidate");
    assert_eq!(
        serde_json::to_value(&native).expect("complete native DTO"),
        serde_json::from_slice::<serde_json::Value>(&packet).expect("actual TS DTO")
    );
    for (provider, candidate) in [
        ("native", native),
        ("typescript", v5::decode_snapshot(&packet).expect("strict TS decode")),
    ] {
        let syntax = v5::verify_snapshot(candidate, &sources).expect("exact syntax owner");
        let entry = sources.verify_file_id(0).expect("original source entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("same-source input"),
        )
        .expect("independent source declarations");
        let bodies = check_execution_body_types(&declarations).expect("genuine body types");
        let instances = semantic::discover(&bodies).expect("closed source instances");
        let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
            .expect("linear layouts");
        let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
            .expect("native layouts");
        let runtime = runtime::verify_v1(
            runtime::raw_v1(&linear, &linux).expect("runtime claim"),
            &linear,
            &linux,
        )
        .expect("independent same-universe runtime");
        let claim =
            semantic::produce_claim(&instances, &linear, &linux).expect("source execution claim");
        let wire = execution_v1::wire::encode(&claim).expect("new execution carrier");
        let program = execution_v1::verify(
            execution_v1::wire::decode(&wire).expect("strict carrier"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("independent NEW execution owner");
        exercise(provider, &program, &wire);
    }
}

fn exercise(provider: &str, program: &VerifiedExecution<'_>, wire: &[u8]) {
    let oracle: serde_json::Value = serde_json::from_slice(&pinned(
        "PRE_EXECUTION_ORACLE.json",
        "bc6fdcf4d69acbd15cd5585b22bdd1a1d193be794c73177e03268d59902f4266",
    ))
    .expect("pre-execution source oracle");
    let kinds = program
        .operations()
        .filter_map(execution_v1::VerifiedOperation::clone_frontier)
        .map(|f| match f.frontier().kind() {
            CloneNodeKind::Copy => "Copy",
            CloneNodeKind::String => "String",
            CloneNodeKind::Struct => "Struct",
            CloneNodeKind::Enum => "Enum",
            CloneNodeKind::FixedArray { .. } => "FixedArray",
            CloneNodeKind::VecEach => "VecEach",
            CloneNodeKind::SharedCount => "SharedCount",
            CloneNodeKind::WeakCount => "WeakCount",
        })
        .collect::<BTreeSet<_>>();
    let expected = oracle["required_Clone_kinds"]
        .as_array()
        .expect("source Clone kinds")
        .iter()
        .map(|s| s.as_str().expect("named Clone kind"))
        .collect::<BTreeSet<_>>();
    assert_eq!(kinds, expected);
    let categories = program
        .layouts(StorageTarget::Linear32V1)
        .types()
        .map(|ty| format!("{:?}", ty.category()))
        .collect::<BTreeSet<_>>();
    assert_eq!(categories.len(), 9);
    let memory_claim =
        zryna_backend_webassembly::generic_execution_v1::memory::derive(program, false)
            .expect("actual recursive memory claim");
    let memory =
        zryna_backend_webassembly::generic_execution_v1::memory::verify(memory_claim, program)
            .expect("independently checked recursive plan");
    assert!(memory.recursive_arena().is_some(), "all catalogue calls use the recursive VM");
    let directory = std::env::temp_dir()
        .join(format!("m416-recursive-catalogue-{}-{provider}", std::process::id()));
    std::fs::create_dir(&directory).expect("new retained actual evidence");
    std::fs::write(directory.join("program.zir"), wire).expect("seal before emission");
    let mut permissions = std::fs::metadata(directory.join("program.zir"))
        .expect("retained wire metadata")
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(directory.join("program.zir"), permissions)
        .expect("read-only same-seal carrier");
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    native_material(program, &directory);
    let ordinary = zryna_backend_webassembly::generic_execution_v1::emit(program)
        .expect("actual ordinary audited Wasm");
    let observed = zryna_backend_webassembly::generic_execution_v1::emit_observed(program)
        .expect("separate audited observed Wasm");
    std::fs::write(directory.join("program.wasm"), ordinary.bytes())
        .expect("actual ordinary bytes");
    std::fs::write(directory.join("program.observed.wasm"), observed.as_bytes())
        .expect("actual observed bytes");
    std::fs::write(
        directory.join("PRE_EXECUTION_ORACLE.json"),
        serde_json::to_vec(&oracle).expect("unchanged oracle values"),
    )
    .expect("frozen oracle before invocation");
    println!("recursive mixed catalogue {provider}: {}", directory.display());
    let mut command = std::process::Command::new("node");
    command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/generic_execution_v1_recursive_owner/mixed.mjs"),
        )
        .arg(&directory);
    let output = process::run(command, 20);
    std::fs::write(directory.join("actual.stdout.json"), &output.stdout)
        .expect("observations before assertions");
    std::fs::write(directory.join("actual.stderr.txt"), &output.stderr)
        .expect("actual errors before assertions");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("retained wire"), wire);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn native_material(program: &VerifiedExecution<'_>, directory: &Path) {
    let mir = zryna_native_mir::generic_execution_v1::lower(program)
        .expect("same-owner mixed recursive native MIR");
    assert!(mir.requires_recursive_arena());
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("supported native object target");
    let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("audited mixed recursive native object");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), &mir)
        .expect("independently checked mixed native inventory");
    std::fs::write(directory.join("program.o"), object.bytes())
        .expect("retain actual mixed native object without execution claim");
    native_mixed::run(program, &mir, object.bytes(), directory);
}
