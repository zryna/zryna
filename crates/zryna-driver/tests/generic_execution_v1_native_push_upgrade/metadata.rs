//! Render canonical runtime element cases from the exact retained new-seal ABI metadata.
use std::{fmt::Write as _, path::Path};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
const SOURCE: &[u8] = include_bytes!("../../../../runtime/native/ownership_runtime_v1.c");
const MARKER: &str = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) {
    assert_eq!(
        super::emit::digest(SOURCE),
        "94ce0a5f0d40458a7dcc234b93f98a2d18c985ff1f60e285c238516bea3d0e26"
    );
    let mut cases = String::new();
    let mut elements = Vec::new();
    for element in program
        .runtime()
        .element_layouts()
        .iter()
        .filter(|row| row.target() == StorageTarget::LinuxX8664V1)
    {
        writeln!(
            cases,
            "case {}U: *stride=UINT64_C({}); *alignment={}U; return 1;",
            element.element().index(),
            element.stride(),
            element.alignment()
        )
        .expect("metadata String write");
        elements.push(serde_json::json!({"element":element.element().index(),"stride":element.stride(),"alignment":element.alignment()}));
    }
    let source = std::str::from_utf8(SOURCE).expect("unchanged canonical runtime UTF8");
    assert_eq!(source.matches(MARKER).count(), 1);
    std::fs::write(directory.join("canonical-runtime.c"), source.replacen(MARKER, &cases, 1))
        .expect("canonical runtime with exact source-seal element cases only");
    let mut header = format!(
        "#include <stdint.h>\nstatic int metadata_element(uint32_t id,uint64_t *stride,uint32_t *alignment) {{ (void)stride;(void)alignment;switch(id) {{ {cases} default:return 0; }} }}\nstatic int metadata_control(uint64_t size,uint32_t alignment) {{ (void)size;(void)alignment;\n"
    );
    let mut controls = Vec::new();
    for control in program
        .runtime()
        .control_layouts()
        .iter()
        .filter(|row| row.target() == StorageTarget::LinuxX8664V1)
    {
        writeln!(
            header,
            "if(size==UINT64_C({}) && alignment=={}U) return 1;",
            control.size(),
            control.alignment()
        )
        .expect("control metadata String write");
        controls.push(serde_json::json!({"payload":control.payload().index(),"payload_offset":control.payload_offset(),"size":control.size(),"alignment":control.alignment()}));
    }
    header.push_str("return 0; }\n");
    std::fs::write(directory.join("runtime_metadata.h"), header)
        .expect("observer reads only verified runtime shapes");
    let receipt = serde_json::json!({"canonical_source_sha256":super::emit::digest(SOURCE),"element_layouts":elements,"control_layouts":controls,"changes":"only exact element marker replaced; no cap/ABI/runtime behavior change"});
    std::fs::write(
        directory.join("runtime_metadata.json"),
        serde_json::to_vec_pretty(&receipt).expect("metadata JSON"),
    )
    .expect("retained checked runtime metadata");
}
