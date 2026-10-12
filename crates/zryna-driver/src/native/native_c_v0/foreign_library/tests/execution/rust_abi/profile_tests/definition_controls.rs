//! Genuine alternate issuers plus explicitly HOSTILE handcrafted ELF metadata, never Rust output.

use super::{OBJECT_SHA, accepted, capture, input, object_fixture::hostile_object};
use crate::native::native_c_v0::{
    foreign_library::capture_rust_profile_library,
    resource_identity::{HandleLinkRequirements, handle_link_requirements},
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use zryna_semantics::native_c_v0::{
    LibraryMaterial,
    body::{compose_private_boundaries, verify_bodies},
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::native_c_source_v0::{AuthenticatedForeignSources, authenticate_sources};

const DECLARATIONS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-rust-v0/declarations.ffi.json"
));
const SCALAR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-rust-v0/source-scalar.zry"
));
const BUFFER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/native-c-rust-v0/source-buffer.zry"
));

#[derive(Clone, Copy)]
enum Variant {
    Missing,
    Renamed,
    Additional,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn wire(mut value: Value) -> Vec<u8> {
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value).expect("canonical hostile document");
    bytes.push(b'\n');
    bytes
}
fn documents(variant: Variant) -> (SourceMap, Value, Vec<u8>) {
    let mut document: Value = serde_json::from_slice(DECLARATIONS).expect("original declarations");
    let mut header = std::str::from_utf8(capture::HEADER).expect("original header").to_owned();
    let mut scalar = SCALAR.to_owned();
    let mut buffer = Some(BUFFER.to_owned());
    let operations = document["operations"].as_array_mut().expect("operations");
    match variant {
        Variant::Missing => {
            operations.retain(|op| op["symbol"] != "sum_bytes");
            buffer = None;
            header = header.replace(
                "int32_t sum_bytes(const uint8_t *bytes, size_t length, int32_t *out);\n",
                "",
            );
        }
        Variant::Renamed => {
            let operation = operations
                .iter_mut()
                .find(|op| op["symbol"] == "sum_bytes")
                .expect("unselected byte operation");
            operation["key"] = format!("{}/sum_other", capture::LIBRARY_ID).into();
            operation["logicalName"] = "sum_other".into();
            operation["symbol"] = "sum_other".into();
            buffer = Some(BUFFER.replace("sum_bytes", "sum_other"));
            header = header.replace("sum_bytes(", "sum_other(");
        }
        Variant::Additional => {
            let mut extra = operations
                .iter()
                .find(|op| op["symbol"] == "add")
                .expect("original add operation")
                .clone();
            extra["key"] = format!("{}/spare", capture::LIBRARY_ID).into();
            extra["logicalName"] = "spare".into();
            extra["symbol"] = "spare".into();
            operations.push(extra);
            scalar.push_str("\nfunction unselected(): i32 { return Ffi.rawCall(\"fixture-rust-c-v0@0/spare\", 0, 0); }\n");
            header =
                header.replace("#endif", "int32_t spare(int32_t left, int32_t right);\n#endif");
        }
    }
    let mut files = vec![SourceFileInput {
        path: "tests/native-c-rust-v0/source-scalar.zry".into(),
        text: scalar,
    }];
    if let Some(buffer) = buffer {
        files.push(SourceFileInput {
            path: "tests/native-c-rust-v0/source-buffer.zry".into(),
            text: buffer,
        });
    }
    (
        SourceMap::build(files).expect("independently authored complete sources"),
        document,
        header.into_bytes(),
    )
}
fn bindings(syntax: &AuthenticatedForeignSources, document: &mut Value) {
    document["sources"] = syntax.files().iter().map(|file| json!({
        "path":file.path(),"sha256":sha(syntax.source_text(file.file_id()).expect("source").as_bytes())
    })).collect::<Vec<_>>().into();
    let mut sites = Vec::new();
    for file in syntax.files() {
        let text = syntax.source_text(file.file_id()).expect("actual source");
        for site in file.sites() {
            let start = usize::try_from(site.span().start()).expect("bounded site start");
            let end = usize::try_from(site.span().end()).expect("bounded site end");
            sites.push(json!({"primitive":site.primitive(),"safety":site.safety(),"operation":site.operation(),
                "path":file.path(),"sourceSha256":sha(text.as_bytes()),"start":start,"end":end,
                "spelling":text.get(start..end).expect("authenticated site")}));
        }
    }
    sites.sort_by(|a, b| {
        (a["path"].as_str(), a["start"].as_u64(), a["end"].as_u64()).cmp(&(
            b["path"].as_str(),
            b["start"].as_u64(),
            b["end"].as_u64(),
        ))
    });
    document["sites"] = sites.into();
    let operations = document["operations"].as_array_mut().expect("operations");
    operations.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    for (ordinal, operation) in operations.iter_mut().enumerate() {
        let key = operation["key"].as_str().expect("key");
        let (file, start, end) = if operation["direction"] == "import" {
            let (file, site) = syntax
                .files()
                .iter()
                .find_map(|file| {
                    file.sites()
                        .iter()
                        .find(|site| {
                            site.operation() == Some(key)
                                && matches!(
                                    site.primitive(),
                                    zryna_syntax::native_c_v0::raw::Primitive::RawCall
                                )
                        })
                        .map(|site| (file, site))
                })
                .expect("genuine alternate import site");
            (file, site.span().start(), site.span().end())
        } else {
            let name = operation["logicalName"].as_str().expect("export name");
            let (file, function) = syntax
                .files()
                .iter()
                .find_map(|file| {
                    file.functions()
                        .iter()
                        .find(|function| function.exported && function.name == name)
                        .map(|function| (file, function))
                })
                .expect("original pure export site");
            (file, function.range.start, function.range.end)
        };
        operation["sourceBinding"] = json!({"path":file.path(),"sha256":sha(syntax.source_text(file.file_id()).expect("source").as_bytes()),"start":start,"end":end,"ordinal":ordinal});
    }
}
fn issue(variant: Variant) -> (HandleLinkRequirements, Vec<u8>, Vec<u8>) {
    let (sources, mut document, header) = documents(variant);
    let syntax = authenticate_sources(&sources).expect("genuine complete alternate syntax");
    bindings(&syntax, &mut document);
    let operations = document["operations"]
        .as_array()
        .expect("operations")
        .iter()
        .filter(|op| op["library"] == capture::LIBRARY_ID)
        .map(|op| {
            let mut policy = op.clone();
            policy.as_object_mut().expect("operation object").remove("sourceBinding");
            policy
        })
        .collect::<Vec<_>>();
    let policy = wire(json!({"allocators":[],"kinds":[],"operations":operations}));
    document["libraries"][0]["headerSha256"] = sha(&header).into();
    document["libraries"][0]["policySha256"] = sha(&policy).into();
    let declarations = zryna_semantics::native_c_v0::verify(
        &wire(document),
        &sources,
        &syntax,
        &[LibraryMaterial {
            library_id: capture::LIBRARY_ID,
            header_bytes: &header,
            policy_bytes: &policy,
        }],
        zryna_syntax::native_c_v0::TARGET,
    )
    .expect("genuine alternate declaration/material issuer");
    let bodies = verify_bodies(&sources, &declarations).expect("complete alternate body issuer");
    let authority = compose_private_boundaries(&sources, &bodies).expect("original private issuer");
    let ir = zryna_native_c_ir::lower(&sources, &authority).expect("genuine alternate IR");
    let mir = zryna_native_mir::native_c_v0::lower(&ir).expect("genuine alternate machine issuer");
    let entry = mir
        .functions()
        .find(|function| function.name() == "imported")
        .expect("unchanged selected add client")
        .entry()
        .symbol
        .clone();
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("target");
    let object =
        zryna_backend_native::native_c_v0::resources::emit_handle_entries(&mir, &[&entry], target)
            .expect("original selected artifact");
    (handle_link_requirements(&object).expect("genuine requirement issuer"), header, policy)
}

#[test]
fn genuine_missing_renamed_and_additional_unselected_definitions_reject_fixed_profile() {
    for (variant, names) in [
        (Variant::Missing, &["add"][..]),
        (Variant::Renamed, &["add", "sum_other"][..]),
        (Variant::Additional, &["add", "spare", "sum_bytes"][..]),
    ] {
        let (requirements, header, policy) = issue(variant);
        assert_eq!(
            requirements
                .object()
                .imported_operations()
                .map(|op| op.declaration().symbol.as_str())
                .collect::<Vec<_>>(),
            ["add"]
        );
        let bytes = hostile_object(names);
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let mut altered = input(&bytes, &digest);
        altered.library.header_bytes = &header;
        altered.library.policy_bytes = &policy;
        assert_eq!(
            capture_rust_profile_library(&requirements, &altered)
                .expect_err(
                    "fixed actual profile refuses a distinct genuine expected definition inventory"
                )
                .code(),
            "ZRYNA-C4104"
        );
        let original = capture::capture();
        let issued = capture::issue(&original);
        let pristine = handle_link_requirements(&issued.scalar_import).expect("original issuer");
        assert_eq!(accepted(&pristine).object_sha256(), &OBJECT_SHA);
    }
}
