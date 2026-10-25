//! Independent complete decoded obligations for concat's distinct private allocator and record adapter.
use super::{A, Diagnostic, Proof};
mod allocation;
mod records;
pub(super) use allocation::allocator;
pub(super) fn prove(p: &mut Proof<'_, '_, '_, '_>) -> Result<(), Diagnostic> {
    for (argument, start) in [(0, 3), (1, 6)] {
        for (offset, local) in [(0, start), (4, start + 1), (8, start + 2)] {
            p.add(&[A(1, argument), A(7, offset), A(2, local)])?;
        }
    }
    for offset in [0, 4, 8] {
        p.add(&[A(1, 2), A(6, 0), A(8, offset)])?;
    }
    records::valid(p, 3)?;
    records::valid(p, 6)?;
    p.add(&[
        A(1, 4),
        A(1, 7),
        A(9, 0),
        A(2, 9),
        A(1, 9),
        A(1, 4),
        A(15, 0),
        A(1, 9),
        A(
            6,
            crate::generic_execution_v1::checked_i64(
                zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES,
            )?,
        ),
        A(16, 0),
        A(18, 0),
        A(21, 0),
    ])?;
    p.fail(3, 2)?;
    p.add(&[A(28, 0), A(1, 9), A(14, 0), A(21, 0), A(6, 0), A(22, 0), A(28, 0)])?;
    let scratch = i64::from(p.model.memory.claims().scratch_start);
    p.add(&[
        A(6, scratch),
        A(1, 9),
        A(6, 1),
        A(31, i64::from(p.model.concat_allocator())),
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
        A(2, 10),
        A(1, 3),
        A(1, 10),
        A(1, 4),
        A(31, 0),
        A(1, 6),
        A(1, 10),
        A(1, 4),
        A(9, 0),
        A(1, 7),
        A(31, 0),
    ])?;
    for (offset, local) in [(0, 10), (4, 9), (8, 9)] {
        p.add(&[A(1, 2), A(1, local), A(8, offset)])?;
    }
    if p.model.observed() {
        p.add(&[
            A(6, 1),
            A(4, 7),
            A(1, 10),
            A(6, 0),
            A(1, 9),
            A(1, 10),
            A(31, i64::from(p.model.recorder)),
        ])?;
    }
    p.add(&[A(6, 0)])
}
