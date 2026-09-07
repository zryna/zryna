//! Independent native MIR lowering and hostile-claim checks.

use zryna_semantics::data_ownership_v1::{SemanticInput, lower as lower_semantics};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

const SOURCE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.zry");
const SNAPSHOT: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.json");

fn fixture() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    fixture_from(SOURCE, SNAPSHOT)
}

fn fixture_from(
    source: &str,
    snapshot: &str,
) -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: source.to_owned(),
    }])
    .expect("source map");
    let syntax = verify_snapshot(decode_snapshot(snapshot.as_bytes()).expect("snapshot"), &sources)
        .expect("verified syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower_semantics(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        .expect("verified DataOwnershipV1")
}

fn verify_rejects(
    raw: zryna_native_mir::data_ownership_v1::raw::Program,
    program: &zryna_semantics::data_ownership_v1::VerifiedProgram,
    code: &str,
) {
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        raw,
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("forged native MIR must fail closed");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == code), "{diagnostics:#?}");
}

#[test]
fn lowering_is_deterministic_layout_bound_and_symbol_sealed() {
    let program = fixture();
    let first = zryna_native_mir::data_ownership_v1::lower_unverified(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("raw MIR");
    let second = zryna_native_mir::data_ownership_v1::lower_unverified(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("replayed MIR");
    assert_eq!(first, second);
    let verified =
        zryna_native_mir::data_ownership_v1::lower(program.verified_ir(), program.runtime_abi())
            .expect("verified MIR");
    let function = verified.functions().next().expect("function");
    assert_eq!(function.identity(), (0, 0));
    assert_eq!(function.symbol(), "zryna_m3_m0_f0");
    assert!(function.place_count() > 0);
    assert!(
        function
            .blocks()
            .map(zryna_native_mir::data_ownership_v1::VerifiedBlock::operation_count)
            .sum::<usize>()
            > 0
    );
    assert_eq!(verified.runtime_symbols().count(), 17);
}

#[test]
fn forged_layout_address_and_runtime_inventory_fail_closed() {
    let program = fixture();
    let raw = zryna_native_mir::data_ownership_v1::lower_unverified(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("raw MIR");

    let mut bad_type = raw.clone();
    bad_type.types[0].size += 1;
    let first = zryna_native_mir::data_ownership_v1::verify(
        bad_type.clone(),
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("forged type");
    let second = zryna_native_mir::data_ownership_v1::verify(
        bad_type,
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("replayed forged type");
    assert_eq!(first, second);
    assert!(first.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3102"));

    let mut bad_address = raw.clone();
    let projection = bad_address.functions[0]
        .places
        .iter_mut()
        .find(|place| {
            matches!(place.kind, zryna_native_mir::data_ownership_v1::raw::PlaceKind::Field { .. })
        })
        .expect("field place");
    if let zryna_native_mir::data_ownership_v1::raw::PlaceKind::Field { offset, .. } =
        &mut projection.kind
    {
        *offset += 1;
    }
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        bad_address,
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("forged address");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3109"));

    let mut bad_cleanup = raw.clone();
    bad_cleanup.functions[0].blocks[0].operations[0].cleanup = Some(u32::MAX);
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        bad_cleanup,
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("forged cleanup reference");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3112"));

    let mut bad_symbols = raw;
    bad_symbols.runtime_symbols[0].push_str("_forged");
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        bad_symbols,
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect_err("forged runtime inventory");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3103"));
}

#[test]
fn forged_operation_call_and_terminator_types_fail_closed() {
    use zryna_native_mir::data_ownership_v1::raw::{
        BorrowAccess, Opcode, Terminator, TypeCategory,
    };

    let program = fixture();
    let raw = zryna_native_mir::data_ownership_v1::lower_unverified(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("raw MIR");
    let bool_type = raw.types.iter().find(|ty| ty.category == TypeCategory::Bool).expect("bool").id;
    let mut bad_arithmetic = raw;
    bad_arithmetic.functions[0]
        .blocks
        .iter_mut()
        .flat_map(|block| &mut block.operations)
        .find(|operation| operation.opcode == Opcode::I32Mul)
        .and_then(|operation| operation.result.as_mut())
        .expect("i32 multiply result")
        .ty = bool_type;
    verify_rejects(bad_arithmetic, &program, "ZRYNA-N3107");

    let call_program = fixture_from(
        include_str!("../../../tests/m3-fixtures/lexical-borrow-call-shared.zry"),
        include_str!("../../../tests/m3-fixtures/lexical-borrow-call-shared.json"),
    );
    let mut bad_call = zryna_native_mir::data_ownership_v1::lower_unverified(
        call_program.verified_ir(),
        call_program.runtime_abi(),
    )
    .expect("call MIR");
    let callee = bad_call
        .functions
        .iter_mut()
        .find(|function| !function.borrow_parameters.is_empty())
        .expect("borrowed callee");
    callee.borrow_parameters[0].access = BorrowAccess::Exclusive;
    verify_rejects(bad_call, &call_program, "ZRYNA-N3107");

    let branch_program = fixture_from(
        include_str!("../../../tests/m3-fixtures/conditional-root-borrow.zry"),
        include_str!("../../../tests/m3-fixtures/conditional-root-borrow.json"),
    );
    let mut bad_return = zryna_native_mir::data_ownership_v1::lower_unverified(
        branch_program.verified_ir(),
        branch_program.runtime_abi(),
    )
    .expect("branch MIR");
    let i32_type =
        bad_return.types.iter().find(|ty| ty.category == TypeCategory::I32).expect("i32").id;
    let function = &mut bad_return.functions[0];
    assert!(
        function.blocks.iter().any(|block| matches!(block.terminator, Terminator::Branch { .. }))
    );
    function.result_type = i32_type;
    verify_rejects(bad_return, &branch_program, "ZRYNA-N3111");
}

#[test]
fn invalid_and_over_budget_string_literals_fail_closed() {
    use zryna_native_mir::data_ownership_v1::raw::{Immediate, Opcode};

    let program = fixture_from(
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.zry"),
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.json"),
    );
    let raw = zryna_native_mir::data_ownership_v1::lower_unverified(
        program.verified_ir(),
        program.runtime_abi(),
    )
    .expect("string MIR");
    let mut invalid = raw.clone();
    let literal = invalid
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.operations)
        .find(|operation| operation.opcode == Opcode::String)
        .expect("String literal");
    literal.immediate = Immediate::Utf8(vec![0xff]);
    verify_rejects(invalid, &program, "ZRYNA-N3107");

    let mut oversized = raw;
    let literal = oversized
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.operations)
        .find(|operation| operation.opcode == Opcode::String)
        .expect("String literal");
    literal.immediate =
        Immediate::Utf8(vec![b'x'; zryna_ir::data_ownership_v1::MAX_STRING_LITERAL_BYTES + 1]);
    verify_rejects(oversized, &program, "ZRYNA-N3201");
}
