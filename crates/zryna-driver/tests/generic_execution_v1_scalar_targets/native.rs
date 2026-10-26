//! Independent native retained-inventory audit; no dead-helper or no-relocation assumption.
use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) -> serde_json::Value {
    let mir = zryna_native_mir::generic_execution_v1::lower(program)
        .expect("same genuine new native owner");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("native target");
    let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("new native scalar object");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), &mir)
        .expect("independent native object inventory");
    let file = object::File::parse(object.bytes()).expect("independent ELF parser");
    let mut expected = mir
        .functions()
        .iter()
        .map(zryna_native_mir::generic_execution_v1::FunctionPlan::symbol)
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(c) = mir.containers() {
        expected.extend(c.clone_symbols().flatten());
        expected.extend(c.drop_symbols().flatten());
    }
    expected.insert("zryna_v1_e_entry");
    let actual = file
        .symbols()
        .filter(|s| s.is_definition() && s.kind() == object::SymbolKind::Text)
        .map(|s| s.name().expect("symbol"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected);
    std::fs::write(directory.join("program.o"), object.bytes()).expect("actual native object");
    let runtime_edges = file
        .sections()
        .flat_map(|section| section.relocations().collect::<Vec<_>>())
        .filter_map(|(_, r)| match r.target() {
            RelocationTarget::Symbol(id) => file
                .symbol_by_index(id)
                .ok()
                .and_then(|s| s.name().ok())
                .filter(|n| n.starts_with("zryna_rt_o1_")),
            _ => None,
        })
        .collect::<Vec<_>>();
    serde_json::json!({"object_sha256":super::emit::digest(object.bytes()),"retained_symbols":actual,"canonical_runtime_relocations":runtime_edges,"independent_inventory_passed":true})
}
