//! Independently authored bytes: no codec, issuer or execution produces this oracle.
use super::{decode, encode};
use crate::generic_v1::execution_v1::raw;
const BOOL_PROGRAM: &str = concat!(
    "5a52594e412d47454e455249432d455845435554494f4e2d56310001000000270000007a72796e612e626f756e646564",
    "2d67656e65726963732d6f7074696f6e2d726573756c742e763101000000000000000100000001000000000000000000",
    "000000000000000000000000000000000000010000000100000000000000000000000000000000000000000000000000",
    "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "000000000000000000000000000000000000000000000000000000000000000100000009000000410000000000000000",
    "000000000000000000000000000000000000000000000000000000000000010000000000000000000000010000000000",
    "000000000000000000000100000000000000000001010001000000000000000001000000000000000000000000000000",
    "0000000000000000",
);
fn literal() -> Vec<u8> {
    BOOL_PROGRAM
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("invalid immutable test hex"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}
#[test]
fn independent_literal_admits_only_raw_claims_and_reencodes_exactly() {
    let bytes = literal();
    let admitted = decode(&bytes).expect("frozen complete literal");
    let claims = admitted.claims();
    assert_eq!(claims.language_contract, raw::CONTRACT);
    assert_eq!(
        claims.functions[0].blocks[0].instructions[0].operation,
        raw::Operation::BoolLiteral(true)
    );
    assert_eq!(encode(claims).expect("bounded raw encoding"), bytes);
}
#[test]
fn every_truncation_and_suffix_rejects() {
    let bytes = literal();
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "{end}");
    }
    let mut suffix = bytes;
    suffix.push(0);
    assert!(decode(&suffix).is_err());
}
#[test]
fn all_older_wire_domains_reject_full_profile_bytes() {
    let bytes = literal();
    assert!(crate::generic_v1::wire::decode(&bytes).is_err());
    assert!(crate::generic_v1::owned_v2::wire::decode(&bytes).is_err());
    assert!(crate::generic_v1::owned_v2::wire::v3::decode(&bytes).is_err());
    assert!(crate::generic_v1::owned_v2::wire::v4::decode(&bytes).is_err());
    assert!(crate::generic_v1::owned_v2::wire::v5::decode(&bytes).is_err());
}
