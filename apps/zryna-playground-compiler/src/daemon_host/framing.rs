//! Bounded exact reads leave the control pipe owned by a single cancellation reader.

use std::{
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) fn read_payload(input: &mut impl Read, limit: usize) -> Result<Vec<u8>, &'static str> {
    let mut header = [0; 4];
    input.read_exact(&mut header).map_err(|_| "PLAYGROUND-REQUEST-IO")?;
    let size =
        usize::try_from(u32::from_le_bytes(header)).map_err(|_| "PLAYGROUND-REQUEST-LIMIT")?;
    if size == 0 || size > limit {
        return Err("PLAYGROUND-REQUEST-LIMIT");
    }
    let mut payload = vec![0; size];
    input.read_exact(&mut payload).map_err(|_| "PLAYGROUND-REQUEST-IO")?;
    Ok(payload)
}

pub(super) fn control(input: &mut impl Read, cancellation: &AtomicBool) {
    let mut byte = [0];
    loop {
        match input.read(&mut byte) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            _ => {
                cancellation.store(true, Ordering::Release);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Partial(io::Cursor<Vec<u8>>);
    impl Read for Partial {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let size = bytes.len().min(1);
            self.0.read(&mut bytes[..size])
        }
    }
    #[test]
    fn partial_reads_preserve_control_byte_and_exact_boundary() {
        let mut bytes = 3_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(b"abc\x01");
        let mut input = Partial(io::Cursor::new(bytes));
        assert_eq!(read_payload(&mut input, 3).expect("exact payload"), b"abc");
        let cancellation = AtomicBool::new(false);
        control(&mut input, &cancellation);
        assert!(cancellation.load(Ordering::Acquire));
    }
    #[test]
    fn first_extra_length_rejects_before_payload_read_or_allocation() {
        let mut bytes = io::Cursor::new(4_u32.to_le_bytes());
        assert_eq!(read_payload(&mut bytes, 3), Err("PLAYGROUND-REQUEST-LIMIT"));
        assert_eq!(bytes.position(), 4);
        assert!(read_payload(&mut io::Cursor::new([0; 4]), 3).is_err());
    }
    #[test]
    fn truncated_header_payload_and_control_eof_fail_closed() {
        assert!(read_payload(&mut io::Cursor::new([1, 0, 0]), 3).is_err());
        assert!(read_payload(&mut io::Cursor::new([2, 0, 0, 0, b'a']), 3).is_err());
        let cancellation = AtomicBool::new(false);
        control(&mut io::empty(), &cancellation);
        assert!(cancellation.load(Ordering::Acquire));
    }
}
