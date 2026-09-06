//! `DataOwnershipV1` native object lowering and audit evidence.

use object::{Object, ObjectSymbol};
use zryna_semantics::data_ownership_v1::{SemanticInput, lower};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

const SOURCE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.zry");
const SNAPSHOT: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.json");

fn verified(source: &str, snapshot: &str) -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: source.to_owned(),
    }])
    .expect("source map");
    let syntax = verify_snapshot(decode_snapshot(snapshot.as_bytes()).expect("snapshot"), &sources)
        .expect("verified syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        .expect("verified DataOwnershipV1")
}

fn fixture() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    verified(SOURCE, SNAPSHOT)
}

fn emit(program: &zryna_semantics::data_ownership_v1::VerifiedProgram) {
    let mir =
        zryna_native_mir::data_ownership_v1::lower(program.verified_ir(), program.runtime_abi())
            .expect("verified native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("target");
    zryna_backend_native::data_ownership_v1::emit_object(&mir, target).expect("native object");
}

#[test]
fn owned_pair_object_is_deterministic_and_runtime_symbol_sealed() {
    let program = fixture();
    let mir =
        zryna_native_mir::data_ownership_v1::lower(program.verified_ir(), program.runtime_abi())
            .expect("verified native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("target");
    let first =
        zryna_backend_native::data_ownership_v1::emit_object(&mir, target).expect("native object");
    let second = zryna_backend_native::data_ownership_v1::emit_object(&mir, target)
        .expect("native object replay");
    assert_eq!(first, second);

    let file = object::File::parse(first.bytes()).expect("ELF object");
    let undefined = file
        .symbols()
        .filter(ObjectSymbol::is_undefined)
        .map(|symbol| symbol.name().expect("symbol"))
        .collect::<Vec<_>>();
    assert_eq!(undefined, mir.runtime_symbols().collect::<Vec<_>>());
    assert!(file.symbol_by_name("zryna_m3_m0_f0").is_some());
}

#[test]
fn exact_operation_inventory_is_retained_for_codegen() {
    let program = fixture();
    let mir =
        zryna_native_mir::data_ownership_v1::lower(program.verified_ir(), program.runtime_abi())
            .expect("verified native MIR");
    let operations = mir
        .functions()
        .flat_map(zryna_native_mir::data_ownership_v1::VerifiedFunction::blocks)
        .flat_map(zryna_native_mir::data_ownership_v1::VerifiedBlock::operations)
        .map(zryna_native_mir::data_ownership_v1::VerifiedOperation::opcode)
        .collect::<Vec<_>>();
    assert!(operations.contains(&zryna_native_mir::data_ownership_v1::raw::Opcode::Construct));
    assert!(operations.contains(&zryna_native_mir::data_ownership_v1::raw::Opcode::Copy));
    assert!(operations.contains(&zryna_native_mir::data_ownership_v1::raw::Opcode::I32Mul));
}

#[test]
fn scalar_root_borrow_addressing_emits() {
    emit(&verified(
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.zry"),
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.json"),
    ));
}

#[test]
fn owned_string_and_vec_borrow_reads_emit() {
    emit(&verified(
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.zry"),
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.json"),
    ));
}

#[test]
fn every_semantically_valid_m3_fixture_emits() {
    let root = std::path::Path::new("../../tests/m3-fixtures");
    let mut failures = Vec::new();
    let mut admitted = 0_usize;
    for entry in std::fs::read_dir(root).expect("fixture directory") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("zry") {
            continue;
        }
        let snapshot = path.with_extension("json");
        if !snapshot.exists() {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source fixture");
        let snapshot = std::fs::read_to_string(snapshot).expect("snapshot fixture");
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "src/main.zry".to_owned(),
            text: source,
        }])
        .expect("source map");
        let Ok(decoded) = decode_snapshot(snapshot.as_bytes()) else {
            continue;
        };
        let Ok(syntax) = verify_snapshot(decoded, &sources) else {
            continue;
        };
        let entry = sources
            .file_id(&NormalizedSourcePath::new("src/main.zry").expect("path"))
            .expect("entry");
        let Ok(program) = lower(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        else {
            continue;
        };
        admitted += 1;
        let mir = zryna_native_mir::data_ownership_v1::lower(
            program.verified_ir(),
            program.runtime_abi(),
        )
        .expect("verified native MIR");
        let target =
            zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
                .expect("target");
        if let Err(error) = zryna_backend_native::data_ownership_v1::emit_object(&mir, target) {
            failures.push((path.file_name().expect("name").to_owned(), error));
        }
    }
    assert!(admitted >= 20, "unexpected admitted fixture count: {admitted}");
    assert!(failures.is_empty(), "native fixture failures: {failures:#?}");
}
