//! Canonical unchanged source and exact i32 element metadata from this NEW seal.
#[path = "../generic_execution_v1_native_runtime/source.rs"]
mod runtime_source;
use std::{fmt::Write as _, path::Path};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
const MARKER: &str = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) {
    let template = runtime_source::template(program);
    assert_eq!(
        super::emit::digest(template.as_bytes()),
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    let mut exports = program.scalar_abi().exports();
    let entry = exports.next().expect("one scalar entry");
    assert!(exports.next().is_none());
    assert_eq!(entry.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(entry.result(), zryna_abi::ScalarType::I32);
    assert!(entry.parameters().is_empty());
    let mut cases = String::new();
    let mut elements = Vec::new();
    for row in program
        .runtime()
        .element_layouts()
        .iter()
        .filter(|r| r.target() == StorageTarget::LinuxX8664V1)
    {
        assert_eq!((row.stride(), row.alignment()), (4, 4));
        writeln!(
            cases,
            "case {}U: *stride=UINT64_C({}); *alignment={}U; return 1;",
            row.element().index(),
            row.stride(),
            row.alignment()
        )
        .expect("metadata write");
        elements.push(serde_json::json!({"element":row.element().index(),"stride":row.stride(),"alignment":row.alignment()}));
    }
    assert_eq!(elements.len(), 1);
    let source = std::str::from_utf8(template.as_bytes()).expect("canonical source UTF8");
    assert_eq!(source.matches(MARKER).count(), 1);
    std::fs::write(directory.join("canonical-runtime.c"), source.replacen(MARKER, &cases, 1))
        .expect("only exact element marker rendered");
    std::fs::write(directory.join("runtime_metadata.h"), format!("#include <stdint.h>\nstatic int metadata_element(uint32_t id,uint64_t *stride,uint32_t *alignment) {{ switch(id) {{ {cases} default:return 0; }} }}\n")).expect("same-seal metadata header");
    std::fs::write(directory.join("runtime_metadata.json"), serde_json::to_vec_pretty(&serde_json::json!({"canonical_source_sha256":super::emit::digest(template.as_bytes()),"element_layouts":elements,"changes":"element marker only; canonical C bytes/policy unchanged"})).expect("metadata JSON")).expect("retained metadata");
}
