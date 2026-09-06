//! Independent native MIR lowering and hostile-claim checks.

use zryna_semantics::data_ownership_v1::{SemanticInput, lower as lower_semantics};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

const SOURCE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.zry");
const SNAPSHOT: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.json");

fn fixture() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: SOURCE.to_owned(),
    }])
    .expect("source map");
    let syntax = verify_snapshot(decode_snapshot(SNAPSHOT.as_bytes()).expect("snapshot"), &sources)
        .expect("verified syntax");
    let entry =
        sources.file_id(&NormalizedSourcePath::new("src/main.zry").expect("path")).expect("entry");
    lower_semantics(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
        .expect("verified DataOwnershipV1")
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
        program.verified_ir().linux_x86_64_layouts(),
        program.runtime_abi(),
    )
    .expect_err("forged type");
    let second = zryna_native_mir::data_ownership_v1::verify(
        bad_type,
        program.verified_ir().linux_x86_64_layouts(),
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
        program.verified_ir().linux_x86_64_layouts(),
        program.runtime_abi(),
    )
    .expect_err("forged address");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3109"));

    let mut bad_cleanup = raw.clone();
    bad_cleanup.functions[0].blocks[0].operations[0].cleanup = Some(u32::MAX);
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        bad_cleanup,
        program.verified_ir().linux_x86_64_layouts(),
        program.runtime_abi(),
    )
    .expect_err("forged cleanup reference");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3112"));

    let mut bad_symbols = raw;
    bad_symbols.runtime_symbols[0].push_str("_forged");
    let diagnostics = zryna_native_mir::data_ownership_v1::verify(
        bad_symbols,
        program.verified_ir().linux_x86_64_layouts(),
        program.runtime_abi(),
    )
    .expect_err("forged runtime inventory");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-N3103"));
}
