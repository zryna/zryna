//! Both input records prove their own exact initialized byte range; no combined-source range is authorized.
use super::{A, Diagnostic, Proof};
pub(super) fn valid(p: &mut Proof<'_, '_, '_, '_>, pointer: i64) -> Result<(), Diagnostic> {
    let length = pointer + 1;
    let capacity = pointer + 2;
    p.add(&[A(1, pointer), A(14, 0), A(21, 0), A(1, length), A(1, capacity), A(18, 0), A(21, 0)])?;
    p.fail(255, 255)?;
    p.add(&[A(28, 0), A(36, 0), A(1, length), A(14, 0), A(21, 0)])?;
    p.fail(255, 255)?;
    p.add(&[
        A(28, 0),
        A(1, pointer),
        A(6, i64::from(p.model.memory.claims().heap_start) + 16),
        A(15, 0),
        A(1, pointer),
        A(4, 0),
        A(17, 0),
        A(18, 0),
        A(1, pointer),
        A(6, 7),
        A(19, 0),
        A(18, 0),
        A(1, pointer),
        A(1, length),
        A(9, 0),
        A(1, pointer),
        A(15, 0),
        A(18, 0),
        A(1, pointer),
        A(1, length),
        A(9, 0),
        A(4, 0),
        A(16, 0),
        A(18, 0),
        A(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.add(&[A(28, 0)])?;
    for (offset, value) in [(8, A(6, 1)), (4, A(1, length))] {
        p.add(&[A(1, pointer), A(6, 16), A(10, 0), A(7, offset), value, A(13, 0), A(21, 0)])?;
        p.fail(255, 255)?;
        p.add(&[A(28, 0)])?;
    }
    p.add(&[A(1, length), A(1, capacity), A(13, 0), A(21, 0)])?;
    p.fail(255, 255)?;
    p.add(&[A(28, 0), A(28, 0)])
}
