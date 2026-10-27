//! Scoped regression for GHSA-qwgh-2vcv-g2f7 on the locked block-buffer 0.10.4.
//! This checks cursor safety after unwinding, not digest correctness after panic.
use sha2::digest::{block_buffer::EagerBuffer, typenum::U4};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn exact_advisory_eager_panic_keeps_separate_cursor() {
    let mut buffer = EagerBuffer::<U4>::new(&[1, 2]);
    let result = catch_unwind(AssertUnwindSafe(|| {
        buffer.digest_blocks(&[3, 255], |_| panic!("compression panic"));
    }));
    assert!(result.is_err());
    assert_eq!(buffer.get_pos(), 2);
    assert_eq!(buffer.get_data(), &[1, 2]);
    buffer.digest_blocks(&[3, 4], |_| {});
    assert_eq!(buffer.get_pos(), 0);
    buffer.digest_blocks(&[5], |_| {});
    assert_eq!(buffer.get_data(), &[5]);
}

#[test]
fn every_partial_position_and_input_byte_preserves_cursor_on_panic() {
    for position in 1..4 {
        for byte in 0..=255 {
            let mut buffer = EagerBuffer::<U4>::new(&[1, 2, 3][..position]);
            let input = vec![byte; 4 - position];
            assert!(catch_unwind(AssertUnwindSafe(|| {
                buffer.digest_blocks(&input, |_| panic!("compression panic"));
            })).is_err());
            assert_eq!(buffer.get_pos(), position);
            assert_eq!(buffer.get_data(), &[1, 2, 3][..position]);
            buffer.reset();
            assert_eq!(buffer.get_pos(), 0);
            buffer.digest_blocks(&[7], |_| {});
            assert_eq!(buffer.get_data(), &[7]);
        }
    }
}

#[test]
fn generator_and_padding_panics_keep_valid_separate_cursors() {
    for position in 0..4 {
        let initial = &[1, 2, 3][..position];
        for panic_at in [1, 2] {
            let mut buffer = EagerBuffer::<U4>::new(initial);
            let mut calls = 0;
            let mut output = vec![0; (4 - position) + 5];
            assert!(catch_unwind(AssertUnwindSafe(|| {
                buffer.set_data(&mut output, |blocks| {
                    calls += 1;
                    for block in blocks { block.fill(255); }
                    if calls == panic_at { panic!("generator panic"); }
                });
            })).is_err());
            assert_eq!(calls, panic_at);
            assert_eq!(buffer.get_pos(), position);
            buffer.reset();
            assert_eq!(buffer.get_pos(), 0);
        }
        for (suffix, panic_at) in [(0, 1), (4, 1), (4, 2)] {
            let mut buffer = EagerBuffer::<U4>::new(initial);
            let mut calls = 0;
            assert!(catch_unwind(AssertUnwindSafe(|| {
                buffer.digest_pad(255, &[255; 4][..suffix], |_| {
                    calls += 1;
                    if calls == panic_at { panic!("padding panic"); }
                });
            })).is_err());
            assert_eq!(calls, panic_at);
            assert_eq!(buffer.get_pos(), position);
            assert_eq!(buffer.get_data().len(), position);
            buffer.reset();
            assert_eq!(buffer.get_pos(), 0);
        }
    }
}
