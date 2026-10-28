//! Seeded Vec boundary evidence uses actual audited bodies and genuine retained source worlds.
use super::super::private_controls::harness::Harness;
use crate::generic_execution_v1::{artifact, code, memory, model::Model};
use sha2::{Digest, Sha256};
mod attacks;
mod clone_prefix;
mod held;
mod invariants;
mod prepared_rhs;
fn with_vec(check: impl FnOnce(&Model<'_, '_, '_>, &[u8])) {
    super::source::with_program("reserve-vec-call-loop", "native", |program, wire| {
        with_model(program, wire, check);
    });
}
fn with_model(
    program: &zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>,
    wire: &[u8],
    check: impl FnOnce(&Model<'_, '_, '_>, &[u8]),
) {
    let memory =
        memory::verify(memory::derive(program, false).expect("genuine memory claim"), program)
            .expect("independent memory plan");
    let m = Model::new(program, &memory).expect("actual retained Vec model");
    assert!(m.accounted(), "genuine Vec operations must share invocation accounting");
    check(&m, wire);
}
fn vector(m: &Model<'_, '_, '_>) -> (u32, i32) {
    let (index, _) = m
        .types
        .iter()
        .enumerate()
        .find(|(_, ty)| ty.category() == zryna_layout::TypeCategory::Vec)
        .expect("actual Vec type");
    let ty = u32::try_from(index).expect("bounded type");
    (
        ty,
        i32::try_from(m.element(ty).expect("exact retained element").element().index())
            .expect("bounded element"),
    )
}
fn scratch(m: &Model<'_, '_, '_>) -> i32 {
    i32::try_from(m.memory.claims().scratch_start).expect("checked scratch")
}
fn oracle() -> serde_json::Value {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../zryna-ownership-runtime-abi/tests/generic_execution_accounting_vec/HELD_COUNTER_EXPECTED_FREEZE.json"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "5c6cbfc840dd06b29a409b5994002c294bf7436dd7d6be7a0abddede76f632f0"
    );
    let addendum = include_bytes!("METADATA_ADDENDUM.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(addendum)),
        "5493da462d23d7b229d40d02f74850663dcc119ea4de8c7f522ab6afeaf480b9"
    );
    serde_json::from_slice(bytes).expect("unchanged eighteen rows")
}

// Private harness record slots use only the retained idle frame interval, never the live heap.
fn resource_records(m: &Model<'_, '_, '_>) -> i32 {
    let start = m.memory.claims().frames_start;
    let end = start.checked_add(48).expect("four exact 12-byte records");
    assert!(
        end <= m.memory.claims().heap_start,
        "private records must fit below the actual live heap"
    );
    assert!(
        start >= m.memory.claims().scratch_start + m.memory.claims().scratch_bytes,
        "records cannot alias raw helper scratch"
    );
    i32::try_from(start).expect("bounded frame address")
}
fn source_literal(m: &Model<'_, '_, '_>, bytes: &[u8]) -> i32 {
    let row =
        m.memory.claims().literals.iter().find(|row| row.bytes.as_slice() == bytes).expect(
            "exact original source literal retained in the independently checked memory plan",
        );
    i32::try_from(row.offset).expect("bounded actual literal address")
}
