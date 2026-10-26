//! Checked same-section offset to the genuine unique private source step; no object mutation.
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, raw};
fn symbol(file: &object::File<'_>, name: &str, local: bool) -> (u64, object::SectionIndex) {
    let rows = file.symbols().filter(|s| s.name() == Ok(name)).collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert!(row.is_definition() && row.kind() == SymbolKind::Text && row.size() > 0);
    assert_eq!(row.is_local(), local);
    let section = row.section_index().expect("concrete code section");
    let bytes = file.section_by_index(section).expect("section");
    assert_eq!(bytes.name().expect("name"), ".text");
    assert!(row.address().checked_add(row.size()).expect("bounded extent") <= bytes.size());
    (row.address(), section)
}
pub(super) fn write(program: &VerifiedExecution<'_>, directory: &Path) {
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("same NEW MIR");
    assert_eq!(mir.invocation_plan().context_bytes(), 32);
    let candidates = mir
        .functions()
        .iter()
        .filter(|p| {
            p.source().parameters().len() == 0
                && p.source().result_type() == raw::ValueType::Stored(1)
                && !program.export_functions().contains(&p.source().index())
        })
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1, "unique same-seal zero-parameter i32 source step");
    let step = candidates[0];
    let bytes = std::fs::read(directory.join("program.o")).expect("ordinary input object");
    let file = object::File::parse(bytes.as_slice()).expect("ELF");
    assert_eq!(file.architecture(), object::Architecture::X86_64);
    assert_eq!(file.endianness(), object::Endianness::Little);
    assert_eq!(file.kind(), object::ObjectKind::Relocatable);
    let mut exports = program.scalar_abi().exports();
    let export = exports.next().expect("actual entry");
    assert!(exports.next().is_none());
    assert_eq!(export.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    assert_eq!(export.parameters(), &[zryna_abi::ScalarType::I32]);
    assert_eq!(export.result(), zryna_abi::ScalarType::I32);
    let public = symbol(&file, export.native_linux_x86_64_symbol().as_str(), false);
    let private = symbol(&file, step.symbol(), true);
    assert_eq!(public.1, private.1, "same unchanged .text input section");
    let difference = i128::from(private.0) - i128::from(public.0);
    assert!(
        difference.unsigned_abs()
            <= u128::try_from(zryna_backend_native::MAX_NATIVE_OBJECT_BYTES)
                .expect("bounded object")
    );
    let delta = i64::try_from(difference).expect("checked signed offset");
    std::fs::write(
        directory.join("private_binding.h"),
        format!("#define M416_STEP_DELTA ((int64_t)({delta}))\n"),
    )
    .expect("test-only exact local binding");
    std::fs::write(directory.join("private-binding.json"),serde_json::to_vec_pretty(&serde_json::json!({
    "object_sha256":super::emit::digest(&bytes),"step_symbol":step.symbol(),"step_delta":delta,
    "public_symbol":export.native_linux_x86_64_symbol().as_str(),"private_abi":"out, appended same invocation context",
    "context_bytes":32,"object_bytes_changed":false,
    "trust":"qualified SystemV x86_64 linker preserves same co-located unchanged .text input-section offsets; no globalize/patch/new public ABI"
 })).expect("binding receipt")).expect("retain checked binding");
    assert_eq!(std::fs::read(directory.join("program.o")).expect("object retained"), bytes);
}
