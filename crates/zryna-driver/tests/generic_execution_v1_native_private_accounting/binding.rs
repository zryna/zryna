//! Checked local/private address relative to the unchanged ordinary public wrapper.
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, VerifiedParameter, raw};
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
fn count(
    plan: &zryna_native_mir::generic_execution_v1::FunctionPlan<'_, '_>,
    clone: bool,
) -> usize {
    plan.source()
        .blocks()
        .flat_map(zryna_ir::generic_v1::execution_v1::VerifiedBlock::operations)
        .filter(|op| {
            if clone {
                matches!(op.kind(), raw::Operation::Clone { .. })
            } else {
                matches!(op.kind(), raw::Operation::StringFromUtf8 { .. })
            }
        })
        .count()
}
fn symbol(file: &object::File<'_>, name: &str, local: bool) -> (u64, object::SectionIndex) {
    let symbols = file.symbols().filter(|symbol| symbol.name() == Ok(name)).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 1);
    let symbol = &symbols[0];
    assert!(symbol.is_definition() && symbol.kind() == SymbolKind::Text && symbol.size() > 0);
    assert_eq!(symbol.is_local(), local);
    let section = symbol.section_index().expect("concrete code section");
    let bytes = file.section_by_index(section).expect("section");
    assert_eq!(bytes.name().expect("section name"), ".text");
    assert!(symbol.address().checked_add(symbol.size()).expect("bounded extent") <= bytes.size());
    (symbol.address(), section)
}
fn delta(file: &object::File<'_>, name: &str, public: (u64, object::SectionIndex)) -> i64 {
    let private = symbol(file, name, true);
    assert_eq!(private.1, public.1, "one unchanged input section, no cross-section guess");
    let difference = i128::from(private.0) - i128::from(public.0);
    assert!(
        difference.unsigned_abs()
            <= u128::try_from(zryna_backend_native::MAX_NATIVE_OBJECT_BYTES)
                .expect("bounded object limit")
    );
    i64::try_from(difference).expect("checked signed same-section delta")
}
pub(super) fn write(case: &str, program: &VerifiedExecution<'_>, directory: &Path) {
    let mir = zryna_native_mir::generic_execution_v1::lower(program).expect("genuine private MIR");
    assert_eq!(mir.invocation_plan().context_bytes(), 32);
    let borrowed = case == "retained-clone-call-loop";
    let steps = mir
        .functions()
        .iter()
        .filter(|plan| {
            count(plan, borrowed) == 1 && plan.source().parameters().len() == usize::from(borrowed)
        })
        .collect::<Vec<_>>();
    assert_eq!(steps.len(), 1, "unique sealed step effect/signature");
    let step = steps[0];
    if borrowed {
        let VerifiedParameter::Loan(loan) = step.source().parameters().next().expect("formal loan")
        else {
            panic!("borrowed step");
        };
        assert_eq!(loan.access(), raw::Access::Shared);
        assert_eq!(
            mir.stored_type(loan.referent()).expect("String").category(),
            TypeCategory::String
        );
    }
    result_i32(&mir, step.source().result_type());
    let consumes = mir.functions().iter().filter(|plan| {
        let mut parameters = plan.source().parameters();
        if parameters.len()!=1 {return false;}
        matches!(parameters.next(),Some(VerifiedParameter::Value(value))
            if matches!(value.ty(),raw::ValueType::Stored(ty) if mir.stored_type(ty).is_some_and(|v|v.category()==TypeCategory::String)))
    }).collect::<Vec<_>>();
    assert_eq!(consumes.len(), 1, "one actual owned String cleanup callee");
    result_i32(&mir, consumes[0].source().result_type());
    let bytes = std::fs::read(directory.join("program.o")).expect("same ordinary emitted object");
    let file = object::File::parse(bytes.as_slice()).expect("ELF");
    assert_eq!(file.architecture(), object::Architecture::X86_64);
    assert_eq!(file.endianness(), object::Endianness::Little);
    assert_eq!(file.kind(), object::ObjectKind::Relocatable);
    let mut exports = program.scalar_abi().exports();
    let export = exports.next().expect("entry");
    assert!(exports.next().is_none());
    assert_eq!(export.parameters(), &[zryna_abi::ScalarType::I32]);
    assert_eq!(export.result(), zryna_abi::ScalarType::I32);
    assert_eq!(export.native_linux_x86_64_symbol().as_str(), "zryna_v1_e_entry");
    let public = symbol(&file, export.native_linux_x86_64_symbol().as_str(), false);
    let step_delta = delta(&file, step.symbol(), public);
    let consume_delta = delta(&file, consumes[0].symbol(), public);
    let header = format!(
        "#include <stdint.h>\n#define M416_PRIVATE_BORROW {}\n#define M416_STEP_DELTA ((int64_t)({step_delta}))\n#define M416_CONSUME_DELTA ((int64_t)({consume_delta}))\n",
        usize::from(borrowed)
    );
    std::fs::write(directory.join("private_binding.h"), header)
        .expect("checked test-only address metadata");
    std::fs::write(directory.join("private-binding.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "object_sha256":super::emit::digest(&bytes),"case":case,"public_symbol":export.native_linux_x86_64_symbol().as_str(),
        "step_symbol":step.symbol(),"consume_symbol":consumes[0].symbol(),"step_delta":step_delta,"consume_delta":consume_delta,
        "source_step_parameters":usize::from(borrowed),"private_parameters":"out, source-order formal records, context LAST",
        "context_bytes":mir.invocation_plan().context_bytes(),"target":"qualified SystemV Linux x86_64",
        "trust":"unchanged ordinary object; trusted linker preserves offsets within identical co-located .text input section; no symbol mutation/newexport/publicFFI"
    })).expect("binding JSON")).expect("retained exact binding");
    assert_eq!(
        super::emit::digest(&std::fs::read(directory.join("program.o")).expect("object retained")),
        super::emit::digest(&bytes)
    );
}
fn result_i32(mir: &VerifiedProgram<'_, '_>, ty: raw::ValueType) {
    let raw::ValueType::Stored(ty) = ty else {
        panic!("scalar result");
    };
    assert_eq!(mir.stored_type(ty).expect("result layout").category(), TypeCategory::I32);
}
