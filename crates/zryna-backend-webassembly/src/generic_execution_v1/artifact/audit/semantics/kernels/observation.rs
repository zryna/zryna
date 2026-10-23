//! Frozen selectors, actual trace reads, bounded byte loop and count commit.
use super::{A, Diagnostic, FunctionBody, Model, need};
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if !m.observed() {
        return Ok(());
    }
    if index == m.observer as usize {
        need(body, &[A(1, 0), A(14, 0), A(21, 0), A(4, 1), A(22, 0), A(28, 0)], m)?;
        need(
            body,
            &[A(1, 0), A(6, -268_435_456), A(19, 0), A(6, 0x2000_0000), A(12, 0), A(21, 0)],
            m,
        )?;
        need(
            body,
            &[
                A(1, 0),
                A(6, 24),
                A(20, 0),
                A(6, 15),
                A(19, 0),
                A(3, 1),
                A(14, 0),
                A(1, 1),
                A(6, 2),
                A(12, 0),
                A(18, 0),
                A(1, 1),
                A(6, 6),
                A(12, 0),
                A(18, 0),
                A(14, 0),
                A(21, 0),
                A(23, 0),
                A(28, 0),
            ],
            m,
        )?;
        need(
            body,
            &[
                A(1, 0),
                A(6, 0x00ff_ffff),
                A(19, 0),
                A(14, 0),
                A(1, 1),
                A(14, 0),
                A(1, 0),
                A(6, 0x2000_0001),
                A(13, 0),
                A(19, 0),
                A(18, 0),
                A(21, 0),
                A(23, 0),
            ],
            m,
        )?;
        need(
            body,
            &[A(1, 0), A(6, 0x4000_0000), A(12, 0), A(21, 0), A(4, 9), A(22, 0), A(28, 0)],
            m,
        )?;
        need(
            body,
            &[
                A(6, i64::from(m.memory.claims().trace_start)),
                A(1, 1),
                A(6, 1),
                A(10, 0),
                A(6, 4),
                A(11, 0),
                A(9, 0),
                A(7, 0),
            ],
            m,
        )?;
    }
    if index == m.recorder as usize {
        need(body, &[A(4, 9), A(6, 4096), A(16, 0), A(21, 0), A(22, 0), A(28, 0)], m)?;
        need(body, &[A(1, 7), A(1, 4), A(17, 0), A(24, 1)], m)?;
        need(body, &[A(1, 7), A(6, 1), A(9, 0), A(2, 7), A(25, 0), A(28, 0), A(28, 0)], m)?;
        need(body, &[A(4, 9), A(1, 4), A(6, 5), A(9, 0), A(9, 0), A(5, 9)], m)?;
    }
    Ok(())
}
