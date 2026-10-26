//! Canonical runtime bytes/header plus same-seal scalar ABI; no fake allocator.
#[path = "../generic_execution_v1_native_runtime/source.rs"]
mod runtime_source;
use super::emit::digest;
use std::{fmt::Write as _, path::Path};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) {
    let mut exports = program.scalar_abi().exports();
    let export = exports.next().expect("one entry");
    assert!(exports.next().is_none());
    assert_eq!(export.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(export.result(), zryna_abi::ScalarType::I32);
    let parameters = export.parameters();
    assert!(parameters.len() <= 2);
    assert!(
        parameters
            .iter()
            .all(|p| matches!(p, zryna_abi::ScalarType::I32 | zryna_abi::ScalarType::Bool))
    );
    let prototype = ["void", "int32_t", "int32_t,int32_t"][parameters.len()];
    std::fs::write(directory.join("entry_abi.h"),format!("#include <stdint.h>\n#define ENTRY_ARGUMENTS {}\nextern int32_t zryna_v1_e_entry({prototype});\n",parameters.len())).expect("same-seal scalar entry header");
    let source = runtime_source::template(program);
    assert_eq!(
        digest(source.as_bytes()),
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    let mut cases = String::new();
    for row in program
        .runtime()
        .element_layouts()
        .iter()
        .filter(|r| r.target() == zryna_layout::StorageTarget::LinuxX8664V1)
    {
        writeln!(
            cases,
            "case {}U: *stride=UINT64_C({}); *alignment={}U; return 1;",
            row.element().index(),
            row.stride(),
            row.alignment()
        )
        .expect("sealed element metadata");
    }
    let source = std::str::from_utf8(source.as_bytes()).expect("canonical UTF8");
    let marker = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
    assert_eq!(source.matches(marker).count(), 1);
    std::fs::write(directory.join("canonical-runtime.c"), source.replacen(marker, &cases, 1))
        .expect("normal unchanged canonical runtime with sealed element cases");
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    std::fs::write(directory.join("zryna_ownership_runtime_v1.h"), header)
        .expect("same exact canonical header");
}
