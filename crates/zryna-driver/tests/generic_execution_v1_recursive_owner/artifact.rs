//! Actual audited object and canonical runtime; only the verified element marker is substituted.
#[path = "../generic_execution_v1_native_runtime/source.rs"]
mod runtime_source;
use std::{fmt::Write as _, path::Path};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;

pub(super) fn native(
    program: &VerifiedExecution<'_>,
    mir: &VerifiedProgram<'_, '_>,
    directory: &Path,
) -> Result<(), Diagnostic> {
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)?;
    let object = zryna_backend_native::generic_execution_v1::emit_object(mir, target)?;
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), mir)?;
    std::fs::write(directory.join("program.o"), object.bytes()).expect("actual audited object");
    let source = runtime_source::template(program);
    assert_eq!(
        super::digest(source.as_bytes()),
        "df9be4eee10a50225414335502f6b8153874b8d6d4b9ea0903932f505d77f980"
    );
    let marker = "/* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */";
    assert_eq!(source.matches(marker).count(), 1);
    let mut cases = String::new();
    for element in program
        .runtime()
        .element_layouts()
        .iter()
        .filter(|e| e.target() == StorageTarget::LinuxX8664V1)
    {
        writeln!(
            cases,
            "case {}U: *stride=UINT64_C({}); *alignment={}U; return 1;",
            element.element().index(),
            element.stride(),
            element.alignment()
        )
        .expect("element cases");
    }
    std::fs::write(directory.join("canonical-runtime.c"), source.replacen(marker, &cases, 1))
        .expect("only element table substituted");
    let header = &program.runtime().declarations().native_header;
    assert_eq!(
        header.as_slice(),
        include_bytes!("../../../zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h")
    );
    std::fs::write(directory.join("zryna_ownership_runtime_v1.h"), header)
        .expect("same-seal canonical header");
    let mut exports = program.scalar_abi().exports();
    let entry = exports.next().expect("entry");
    assert!(exports.next().is_none());
    assert_eq!(entry.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(entry.result(), zryna_abi::ScalarType::I32);
    assert_eq!(entry.parameters(), &[zryna_abi::ScalarType::I32]);
    Ok(())
}
