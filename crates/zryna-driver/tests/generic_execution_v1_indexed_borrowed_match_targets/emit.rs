//! No target can substitute another source proof; canonical runtime replacement is the element table only.
#[path = "../generic_execution_v1_native_runtime/source.rs"]
mod runtime_source;
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn all(
    case: &str,
    provider: &str,
    p: &VerifiedExecution<'_>,
    wire: &[u8],
    oracle: &[u8],
) -> PathBuf {
    let directory = std::env::temp_dir()
        .join(format!("m416-indexed-borrowed-targets-{}-{case}-{provider}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh actual target directory");
    std::fs::write(directory.join("program.zir"), wire)
        .expect("retain source seal before first target emission");
    let mut permissions =
        std::fs::metadata(directory.join("program.zir")).expect("wire metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(directory.join("program.zir"), permissions)
        .expect("read-only source wire before targets");
    std::fs::write(directory.join("SCALAR_EXPECTED.json"), oracle)
        .expect("retain pre-output oracle");
    let js =
        zryna_backend_javascript::generic_execution_v1::emit(p).expect("same-seal ordinary JS");
    let wasm =
        zryna_backend_webassembly::generic_execution_v1::emit(p).expect("same-seal core Wasm");
    let mir = zryna_native_mir::generic_execution_v1::lower(p).expect("same-seal native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("qualified native target");
    let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("same-seal native object");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), &mir)
        .expect("independent object owner");
    for (name, bytes) in [
        ("program.mjs", js.source.as_bytes()),
        ("program.wasm", wasm.bytes()),
        ("program.o", object.bytes()),
    ] {
        std::fs::write(directory.join(name), bytes).expect("retain actual target bytes");
    }
    runtime(p, &directory);
    assert_eq!(std::fs::read(directory.join("program.zir")).expect("retained genuine wire"), wire);
    let files = [
        "program.zir",
        "program.mjs",
        "program.wasm",
        "program.o",
        "canonical-runtime.c",
        "SCALAR_EXPECTED.json",
    ]
    .into_iter()
    .map(|name| (name, digest(&std::fs::read(directory.join(name)).expect("retained bytes"))))
    .collect::<std::collections::BTreeMap<_, _>>();
    std::fs::write(directory.join("same-seal-artifacts.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "case":case,"provider":provider,"wire_sha256":digest(wire),"single_verified_execution":true,
        "files_sha256":files,"limits":"Scalar/bounds/actual native malloc-free only, no full17 observer or recursive qualification claim."
    })).expect("receipt")).expect("retain source/target binding");
    directory
}

fn runtime(p: &VerifiedExecution<'_>, directory: &Path) {
    let source = runtime_source::template(p);
    assert_eq!(
        digest(source.as_bytes()),
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    let mut cases = String::new();
    for element in
        p.runtime().element_layouts().iter().filter(|e| e.target() == StorageTarget::LinuxX8664V1)
    {
        writeln!(
            cases,
            "case {}U: *stride=UINT64_C({}); *alignment={}U; return 1;",
            element.element().index(),
            element.stride(),
            element.alignment()
        )
        .expect("exact checked element cases");
    }
    let marker = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
    let source = std::str::from_utf8(source.as_bytes()).expect("canonical UTF8");
    assert_eq!(source.matches(marker).count(), 1);
    std::fs::write(directory.join("canonical-runtime.c"), source.replacen(marker, &cases, 1))
        .expect("only canonical element marker replaced");
    let header = &p.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    std::fs::write(directory.join("zryna_ownership_runtime_v1.h"), header)
        .expect("same-seal runtime header");
    let mut exports = p.scalar_abi().exports();
    let entry = exports.next().expect("scalar entry");
    assert!(exports.next().is_none());
    assert_eq!(entry.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(entry.result(), zryna_abi::ScalarType::I32);
    let args = entry.parameters();
    assert!((1..=2).contains(&args.len()) && args.iter().all(|t| *t == zryna_abi::ScalarType::I32));
    let prototype = vec!["int32_t"; args.len()].join(",");
    let call =
        if args.len() == 1 { "argument(argv[1])" } else { "argument(argv[1]),argument(argv[2])" };
    let observer = include_str!("observer.c")
        .replace("ENTRY_ARGUMENT_COUNT", &args.len().to_string())
        .replace("ENTRY_PROTOTYPE", &prototype)
        .replace("ENTRY_CALL", call);
    std::fs::write(directory.join("observer.c"), observer)
        .expect("exact checked scalar ABI observer");
}
