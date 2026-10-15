//! Real source, syntax, header and policy capture; never constructs a verified value.
use super::*;
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use zryna_source::SourceFileInput;

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn wire(mut value: serde_json::Value) -> Vec<u8> {
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec(&value).expect("canonical independently authored capture");
    bytes.push(b'\n');
    bytes
}

pub(super) fn imports(library: usize, count: usize, tag: usize) -> NativeAuthority {
    let names = (0..count).map(|index| format!("import_{library:03}_{index:03}")).collect();
    make(library, names, tag, false, "")
}
pub(super) fn exports(module: usize, count: usize, tag: usize) -> NativeAuthority {
    let names = (0..count).map(|index| format!("export_{module:03}_{index:03}")).collect();
    make(module, names, tag, true, "")
}
pub(super) fn make(
    library: usize,
    names: Vec<String>,
    tag: usize,
    export: bool,
    header_suffix: &str,
) -> NativeAuthority {
    let id = format!("library{library:03}@0");
    let module = format!("module{library:03}@0");
    let path = format!("closure/capture{tag:03}.zry");
    let template: serde_json::Value =
        serde_json::from_slice(DECLARATIONS).expect("original raw templates");
    let prototype = template["operations"]
        .as_array()
        .expect("operations")
        .iter()
        .find(|operation| operation["direction"] == if export { "export" } else { "import" })
        .expect("scalar operation template");
    let mut text = String::new();
    let mut operations = Vec::new();
    let mut sites = Vec::new();
    let mut header = String::from("#include <stdint.h>\n");
    let operation_count = names.len();
    for (ordinal, name) in names.into_iter().enumerate() {
        let key = format!("{}/{name}", if export { &module } else { &id });
        let call = format!("Ffi.rawCall(\"{key}\", left, right)");
        let function = if export {
            format!(
                "export function {name}(left: i32, right: i32): i32 {{ return left + right; }}\n"
            )
        } else {
            format!("function call{ordinal:03}(left: i32, right: i32): i32 {{ return {call}; }}\n")
        };
        let start = text.len();
        text.push_str(&function);
        let end = text.len() - 1;
        let (binding_start, binding_end) = if export {
            (start, end)
        } else {
            let call_start = start + function.find("Ffi.rawCall(").expect("actual call location");
            (call_start, call_start + call.len())
        };
        let mut operation = prototype.clone();
        operation["key"] = key.clone().into();
        operation["library"] = if export { module.clone() } else { id.clone() }.into();
        operation["logicalName"] = name.clone().into();
        operation["symbol"] =
            if export { format!("zryna_c_v0_e_{name}") } else { name.clone() }.into();
        operation["sourceBinding"] = serde_json::json!({"path":path,"sha256":"pending","start":binding_start,"end":binding_end,"ordinal":ordinal});
        operations.push(operation);
        if !export {
            sites.push(serde_json::json!({"primitive":"rawCall","safety":"unsafe-raw","operation":key,
                "path":path,"sourceSha256":"pending","start":binding_start,"end":binding_end,"spelling":call}));
            writeln!(header, "int32_t {name}(int32_t left, int32_t right);")
                .expect("captured header text");
        }
    }
    header.push_str(header_suffix);
    let digest = sha(text.as_bytes());
    for operation in &mut operations {
        operation["sourceBinding"]["sha256"] = digest.clone().into();
    }
    for site in &mut sites {
        site["sourceSha256"] = digest.clone().into();
    }
    let mut document = template;
    document["sources"] = serde_json::json!([{"path":path,"sha256":digest}]);
    document["operations"] = operations.clone().into();
    document["sites"] = sites.into();
    let mut policies = operations;
    for policy in &mut policies {
        policy.as_object_mut().expect("operation").remove("sourceBinding");
    }
    let policy = wire(serde_json::json!({"allocators":[],"kinds":[],"operations":policies}));
    document["libraries"] = if export {
        serde_json::json!([])
    } else {
        serde_json::json!([{"id":id,"version":"0","headerSha256":sha(header.as_bytes()),"policySha256":sha(&policy),"kinds":[],"allocators":[]}])
    };
    let sources =
        SourceMap::build(vec![SourceFileInput { path, text }]).expect("new actual source issuer");
    let syntax = zryna_syntax::native_c_source_v0::authenticate_sources(&sources)
        .expect("complete independent syntax authentication");
    let materials = if export {
        Vec::new()
    } else {
        vec![LibraryMaterial {
            library_id: &id,
            header_bytes: header.as_bytes(),
            policy_bytes: &policy,
        }]
    };
    let declarations = zryna_semantics::native_c_v0::verify_report(
        &wire(document),
        &sources,
        &syntax,
        &materials,
        zryna_syntax::native_c_v0::TARGET,
    )
    .expect("genuine source/material declaration authentication");
    assert_eq!(declarations.operation_count(), operation_count);
    NativeAuthority { sources, declarations }
}
