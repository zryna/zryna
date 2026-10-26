//! Bind no-argument scalar ABI, original object, generic runtime and test-only local TLS projection.
use object::{Object, ObjectSymbol};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, path::Path};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn prepare(
    program: &VerifiedExecution<'_>,
    mir: &VerifiedProgram<'_, '_>,
    bytes: &[u8],
    original: &[u8],
    oracle: &Value,
    directory: &Path,
) {
    zryna_backend_native::generic_execution_v1::validate_object_inventory(bytes, mir)
        .expect("original same-owner object remains audited");
    assert_eq!(std::fs::read(directory.join("program.o")).expect("retained object"), bytes);
    let mut exports = program.scalar_abi().exports();
    let entry = exports.next().expect("actual entry");
    assert!(exports.next().is_none());
    assert!(entry.parameters().is_empty(), "native entry has NO arguments");
    assert_eq!(entry.result(), zryna_abi::ScalarType::I32);
    assert_eq!(entry.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(oracle["ordinary_return_i32"], 97);
    super::metadata::write(program, directory);
    std::fs::write(
        directory.join("zryna_ownership_runtime_v1.h"),
        &program.runtime().declarations().native_header,
    )
    .expect("same checked17ABI header");
    let file = object::File::parse(bytes).expect("actual immutable ELF");
    let address = |name: &str| {
        file.symbols()
            .find(|symbol| symbol.name().ok() == Some(name))
            .expect("audited symbol")
            .address()
    };
    let delta = i64::try_from(
        i128::from(address("zryna_gexecution_tls_address"))
            - i128::from(address("zryna_v1_e_entry")),
    )
    .expect("checked private local projection");
    std::fs::write(
        directory.join("private_binding.h"),
        format!("#define M416_TLS_DELTA INT64_C({delta})\n"),
    )
    .expect("TLS projection only");
    std::fs::write(directory.join("native-PRE_EXECUTION_ORACLE.json"), original)
        .expect("byte-exact pre-execution oracle before invocation");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_recursive_owner/native_mixed");
    for name in
        ["observer.h", "observer.c", "payload.c", "wrappers.c", "alloc.h", "counts.h", "plain.c"]
    {
        std::fs::copy(root.join(name), directory.join(format!("mixed-{name}")))
            .expect("retain exact private test inputs");
    }
    payloads(program, oracle, directory);
}
fn payloads(program: &VerifiedExecution<'_>, oracle: &Value, directory: &Path) {
    use zryna_layout::{StorageTarget, TypeCategory};
    let types = program.layouts(StorageTarget::LinuxX8664V1).types().collect::<Vec<_>>();
    let string =
        types.iter().find(|ty| ty.category() == TypeCategory::String).expect("String type").id();
    let node = types
        .iter()
        .find(|ty| {
            ty.category() == TypeCategory::Enum
                && ty.variants().any(|(_, child)| {
                    child.is_some_and(|id| {
                        types.iter().any(|child| {
                            child.id() == id
                                && child.category() == TypeCategory::Vec
                                && child.referenced_type() == Some(ty.id())
                        })
                    })
                })
        })
        .expect("genuine Node/Vec recursive edge")
        .id();
    let control = program
        .runtime()
        .control_layouts()
        .iter()
        .find(|row| row.target() == StorageTarget::LinuxX8664V1 && row.payload() == string)
        .expect("String control");
    let rows = oracle["pristine_allocations"].as_array().expect("26 original rows");
    assert_eq!(rows.len(), 26);
    let mut out = String::from(
        "typedef struct { unsigned kind, length, source; uint8_t bytes[7]; } expected_row;\nstatic const expected_row expected_rows[27]={{0,0,0,{0}},\n",
    );
    for row in rows {
        let kind = match row["kind"].as_str().expect("source kind") {
            "String" => 1,
            "Vec" => 2,
            "Control" => 3,
            _ => panic!("unknown source kind"),
        };
        let bytes = row["utf8"].as_array().map_or_else(Vec::new, |bytes| {
            bytes.iter().map(|byte| byte.as_u64().expect("source byte").to_string()).collect()
        });
        assert!(bytes.len() <= 7);
        let source = row
            .get("source_id")
            .or_else(|| row.get("payload_string_id"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        writeln!(
            out,
            "{{{kind},{},{source},{{{}}}}},",
            bytes.len(),
            if bytes.is_empty() { "0".into() } else { bytes.join(",") }
        )
        .expect("fixed source table");
    }
    out.push_str("};\n");
    writeln!(out, "#define MIXED_STRING_ELEMENT {}U\n#define MIXED_NODE_ELEMENT {}U\n#define MIXED_CONTROL_SIZE UINT64_C({})\n#define MIXED_CONTROL_ALIGNMENT {}U",
        string.index(), node.index(), control.size(), control.alignment()).expect("sealed metadata");
    std::fs::write(directory.join("native_payloads.h"), out)
        .expect("source-derived fixed guard table");
}
