//! Capacity choice and preservation proof for each actual canonical element record.
use super::{A, Diagnostic, Proof};
use crate::generic_execution_v1::model::containers as C;
use zryna_layout::StorageTarget;
pub(super) fn prove(proof: &mut Proof<'_, '_, '_, '_>) -> Result<(), Diagnostic> {
    for (offset, local) in [(0, 4), (4, 5), (8, 6)] {
        proof.add(&[A(1, 1), A(7, offset), A(2, local)])?;
    }
    for offset in [0, 4, 8] {
        proof.add(&[A(1, 3), A(6, 0), A(8, offset)])?;
    }
    proof.add(&[A(1, 5), A(1, 6), A(16, 0), A(21, 0)])?;
    proof.fail(255, 255)?;
    proof.add(&[A(28, 0)])?;
    let rows = proof.model.program.runtime().element_layouts();
    proof.model.take(rows.len())?;
    for record in rows.iter().filter(|record| record.target() == StorageTarget::Linear32V1) {
        let ceiling = zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
            .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / record.stride());
        let ceiling = crate::generic_execution_v1::checked_i64(ceiling)?;
        let stride =
            i64::from(crate::generic_execution_v1::checked_u32(record.stride())?.cast_signed());
        let align =
            i64::from(crate::generic_execution_v1::checked_u32(record.alignment())?.cast_signed());
        proof.add(&[A(1, 0), A(6, i64::from(record.element().index())), A(12, 0), A(21, 0)])?;
        limits(proof, ceiling, stride, align)?;
        candidate(proof, ceiling)?;
        acquire_and_publish(proof, stride, align)?;
        proof.add(&[A(28, 0)])?;
    }
    proof.fail(255, 255)
}
fn limits(
    proof: &mut Proof<'_, '_, '_, '_>,
    ceiling: i64,
    stride: i64,
    align: i64,
) -> Result<(), Diagnostic> {
    for local in [6, 2] {
        proof.add(&[A(1, local), A(6, ceiling), A(16, 0), A(21, 0)])?;
        if local == 6 {
            proof.fail(255, 255)?;
        } else {
            proof.fail(3, 2)?;
        }
        proof.add(&[A(28, 0)])?;
    }
    proof.add(&[A(1, 6), A(14, 0), A(21, 0), A(1, 4), A(21, 0)])?;
    proof.fail(255, 255)?;
    proof.add(&[A(28, 0), A(36, 0)])?;
    proof.valid(4)?;
    proof.add(&[
        A(1, 4),
        A(6, 16),
        A(10, 0),
        A(7, 4),
        A(1, 6),
        A(6, stride),
        A(11, 0),
        A(13, 0),
        A(21, 0),
    ])?;
    proof.fail(255, 255)?;
    proof.add(&[
        A(28, 0),
        A(1, 4),
        A(6, 16),
        A(10, 0),
        A(7, 12),
        A(6, align),
        A(13, 0),
        A(21, 0),
    ])?;
    proof.fail(255, 255)?;
    proof.add(&[
        A(28, 0),
        A(28, 0),
        A(1, 2),
        A(1, 6),
        A(32, 0),
        A(21, 0),
        A(1, 1),
        A(1, 3),
        A(6, 12),
        A(31, 0),
        A(6, 0),
        A(22, 0),
        A(28, 0),
    ])
}
fn candidate(proof: &mut Proof<'_, '_, '_, '_>, ceiling: i64) -> Result<(), Diagnostic> {
    // Candidate doubles strictly, or exits to exact-required on the checked canonical bound.
    proof.add(&[
        A(1, 6),
        A(14, 0),
        A(21, 0),
        A(6, 4),
        A(2, 7),
        A(36, 0),
        A(1, 6),
        A(6, 2),
        A(11, 0),
        A(2, 7),
        A(28, 0),
        A(30, 0),
        A(29, 0),
        A(1, 7),
        A(1, 2),
        A(17, 0),
        A(24, 1),
        A(1, 7),
        A(6, ceiling / 2),
        A(16, 0),
        A(21, 0),
        A(1, 2),
        A(2, 7),
        A(25, 2),
        A(28, 0),
        A(1, 7),
        A(6, 2),
        A(11, 0),
        A(2, 7),
        A(25, 0),
        A(28, 0),
        A(28, 0),
        A(1, 7),
        A(6, ceiling),
        A(16, 0),
        A(21, 0),
        A(1, 2),
        A(2, 7),
        A(28, 0),
    ])
}
fn acquire_and_publish(
    proof: &mut Proof<'_, '_, '_, '_>,
    stride: i64,
    align: i64,
) -> Result<(), Diagnostic> {
    let scratch = i64::from(proof.model.memory.claims().scratch_start);
    let allocate = i64::from(proof.model.container(C::ALLOCATE_RAW));
    let release = i64::from(proof.model.container(C::RELEASE_RAW));
    proof.add(&[
        A(6, scratch),
        A(1, 7),
        A(6, stride),
        A(11, 0),
        A(6, align),
        A(31, allocate),
        A(35, 0),
        A(6, scratch + 4),
        A(7, 0),
        A(21, 0),
        A(6, scratch + 4),
        A(7, 0),
        A(22, 0),
        A(28, 0),
        A(6, scratch),
        A(7, 0),
        A(2, 8),
        A(1, 4),
        A(1, 8),
        A(1, 5),
        A(6, stride),
        A(11, 0),
        A(31, 0),
        A(1, 4),
        A(1, 6),
        A(6, stride),
        A(11, 0),
        A(6, align),
        A(31, release),
        A(21, 0),
        A(23, 0),
        A(28, 0),
    ])?;
    for (offset, local) in [(0, 8), (4, 5), (8, 7)] {
        proof.add(&[A(1, 3), A(1, local), A(8, offset)])?;
    }
    proof.add(&[A(6, 0), A(22, 0)])
}
