//! Actual immutable old-codec bytes remain accepted only by their existing wire domain.
use super::{HEADER, VERSION, decode};
use crate::generic_v1::{owned_v2::wire as owned, wire as core};
fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<_> = text.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    assert!(digits.len().is_multiple_of(2));
    digits
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("immutable old fixture hex"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}
fn retag(body: &[u8]) -> Vec<u8> {
    let mut bytes = HEADER.to_vec();
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
#[test]
fn actual_v1_to_v4_literals_keep_old_admission_and_reject_new_or_retagged_admission() {
    let cases = [
        (
            include_str!("../../../../../../tests/m7-generic-struct-v4/embedded-core-v1.hex"),
            core::HEADER,
            core::VERSION,
        ),
        (
            include_str!("../../../../../../tests/m7-generic-struct-v4/wire-owned-v2.hex"),
            owned::HEADER,
            owned::VERSION,
        ),
        (
            include_str!("../../../../../../tests/m7-generic-struct-v4/wire-owned-v3.hex"),
            owned::v3::HEADER,
            owned::v3::VERSION,
        ),
        (
            include_str!("../../../../../../tests/m7-generic-struct-v4/wire-v4.hex"),
            owned::v4::HEADER,
            owned::v4::VERSION,
        ),
    ];
    for (index, (text, header, version)) in cases.into_iter().enumerate() {
        let bytes = hex(text);
        match index {
            0 => assert!(core::decode(&bytes).is_ok()),
            1 => assert!(owned::decode(&bytes).is_ok()),
            2 => assert!(owned::v3::decode(&bytes).is_ok()),
            3 => assert!(owned::v4::decode(&bytes).is_ok()),
            _ => unreachable!(),
        }
        assert_eq!(&bytes[..header.len()], header);
        assert_eq!(&bytes[header.len()..header.len() + 4], version.to_le_bytes());
        assert!(decode(&bytes).is_err());
        assert!(decode(&retag(&bytes[header.len() + 4..])).is_err());
    }
}
#[test]
fn inherited_v5_payload_control_stays_v5_only_even_after_new_envelope_retag() {
    let v4 = hex(include_str!("../../../../../../tests/m7-generic-struct-v4/wire-v4.hex"));
    let body = &v4[owned::v4::HEADER.len() + 4..];
    let mut v5 = owned::v5::HEADER.to_vec();
    v5.extend_from_slice(&owned::v5::VERSION.to_le_bytes());
    v5.extend_from_slice(body);
    assert!(owned::v5::decode(&v5).is_ok());
    assert!(decode(&v5).is_err());
    assert!(decode(&retag(body)).is_err());
}
