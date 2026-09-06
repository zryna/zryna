//! Executed and audited `DataOwnershipV1` core WebAssembly checks.

use std::process::Command;

use zryna_semantics::data_ownership_v1::{SemanticInput, lower};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

const SOURCE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.zry");
const SNAPSHOT: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.json");

fn shift_spans(value: &mut serde_json::Value, cutoff: u64) {
    match value {
        serde_json::Value::Object(object)
            if object.contains_key("file")
                && object.contains_key("start")
                && object.contains_key("end") =>
        {
            for key in ["start", "end"] {
                let current = object[key].as_u64().expect("span offset");
                if current >= cutoff {
                    object.insert(key.to_owned(), (current + 7).into());
                }
            }
        }
        serde_json::Value::Object(object) => {
            for child in object.values_mut() {
                shift_spans(child, cutoff);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                shift_spans(child, cutoff);
            }
        }
        _ => {}
    }
}

fn exported(
    source_text: &str,
    snapshot: &str,
) -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let cutoff =
        u64::try_from(source_text.find("function").expect("function keyword")).expect("offset");
    let mut value: serde_json::Value = serde_json::from_str(snapshot).expect("snapshot JSON");
    shift_spans(&mut value, cutoff);
    value["files"][0]["functions"][0]["span"]["start"] = cutoff.into();
    value["files"][0]["functions"][0]["export_span"] = serde_json::json!({
        "file": 0, "start": cutoff, "end": cutoff + 6
    });
    let mut source = source_text.to_owned();
    source.insert_str(usize::try_from(cutoff).expect("offset"), "export ");
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "src/main.zry".to_owned(), text: source }])
            .expect("source map");
    let raw =
        decode_snapshot(&serde_json::to_vec(&value).expect("snapshot bytes")).expect("snapshot");
    let syntax = verify_snapshot(raw, &sources).expect("verified syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower(SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"))
        .expect("verified DataOwnershipV1")
}

fn fixture() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    exported(SOURCE, SNAPSHOT)
}

fn private(source: &str, snapshot: &str) -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: source.to_owned(),
    }])
    .expect("source map");
    let syntax = verify_snapshot(decode_snapshot(snapshot.as_bytes()).expect("snapshot"), &sources)
        .expect("syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        .expect("verified DataOwnershipV1")
}

#[test]
fn memory_bearing_aggregate_module_is_deterministic_and_executes() {
    let program = fixture();
    let first = zryna_backend_webassembly::emit_data_ownership(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("wasm");
    let second = zryna_backend_webassembly::emit_data_ownership(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("wasm replay");
    assert_eq!(first, second);
    let bytes = first.bytes().iter().map(u8::to_string).collect::<Vec<_>>().join(",");
    let script = format!(
        "const b=new Uint8Array([{bytes}]);const m=await WebAssembly.instantiate(b);const f=Object.values(m.instance.exports)[0];console.log(f(2,3));"
    );
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8"), "65\n");
}

#[test]
fn private_exclusive_borrow_address_module_validates() {
    let program = private(
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.zry"),
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.json"),
    );
    let artifact = zryna_backend_webassembly::emit_data_ownership(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("wasm");
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM1)
        .validate_all(artifact.bytes())
        .expect("valid private borrow module");
}

#[test]
fn module_has_one_private_bounded_memory_and_no_imports() {
    let program = fixture();
    let artifact = zryna_backend_webassembly::emit_data_ownership(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("wasm");
    let mut memories = 0;
    for payload in wasmparser::Parser::new(0).parse_all(artifact.bytes()) {
        match payload.expect("valid payload") {
            wasmparser::Payload::ImportSection(section) => assert_eq!(section.count(), 0),
            wasmparser::Payload::MemorySection(section) => memories += section.count(),
            wasmparser::Payload::ExportSection(section) => {
                assert!(
                    section
                        .into_iter()
                        .all(|item| item.expect("export").kind == wasmparser::ExternalKind::Func)
                );
            }
            _ => {}
        }
    }
    assert_eq!(memories, 1);
}
