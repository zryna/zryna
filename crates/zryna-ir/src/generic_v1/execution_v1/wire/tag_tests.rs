//! Hand-authored payload bytes are independent of encoder output and any issuer.
use super::{Failure, reader::Reader, writer::Writer};
use std::fmt::Debug;
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("immutable test hex"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}
fn verify<T: Debug + PartialEq>(
    expected: &T,
    literal: &str,
    read: impl Fn(&mut Reader<'_>) -> Result<T, Failure>,
    write: impl Fn(&mut Writer, &T) -> Result<(), Failure>,
) {
    let bytes = hex(literal);
    let mut reader = Reader::new(&bytes);
    assert_eq!(&read(&mut reader).expect("independent literal"), expected);
    assert_eq!(reader.cursor, bytes.len());
    let mut writer = Writer::new();
    write(&mut writer, expected).expect("bounded payload");
    assert_eq!(writer.bytes, bytes);
    for end in 0..bytes.len() {
        assert!(read(&mut Reader::new(&bytes[..end])).is_err(), "{literal} truncation {end}");
    }
}
macro_rules! check {
    ($method:ident, $expected:expr, $literal:literal) => {
        super::verify(&$expected, $literal, super::read::$method, super::Writer::$method)
    };
}
macro_rules! check_copy {
    ($method:ident, $expected:expr, $literal:literal) => {
        super::verify(&$expected, $literal, super::read::$method, |writer, value| {
            writer.$method(*value)
        })
    };
}
mod control;
mod model;
mod operations;
mod read;
