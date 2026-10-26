//! Fixed bytes, exact decoding and version/namespace isolation.
use super::claims::{
    CORE, FROZEN_COPY, OLD_V2, OLD_V3, VECTOR, empty_claim, literal_claim, rebrand,
};
use zryna_ir::generic_v1::{Failure, owned_v2 as owned, wire as frozen};

#[test]
fn independently_literal_v4_is_exact_and_preserves_frozen_copy_v1() {
    // This typed binding is also the compile-pass control for the laundering doctests.
    let _: fn(&[u8]) -> Result<owned::wire::v4::DecodedOwnedV4, Failure> = owned::wire::v4::decode;
    assert_eq!(VECTOR.len(), 440);
    assert_eq!(CORE.len(), 343);
    assert_eq!(FROZEN_COPY.len(), 277);
    let c = literal_claim();
    assert_eq!(owned::wire::v4::encode(&c).expect("v4 raw encoder"), VECTOR);
    assert_eq!(owned::wire::v4::decode(VECTOR).expect("literal raw decoder").claims(), &c);
    assert_eq!(frozen::encode(&c.graph).expect("frozen embedded graph"), CORE);
    let old = frozen::decode(FROZEN_COPY).expect("independent old vector");
    assert_eq!(frozen::encode(old.claims()).expect("unchanged old encoder"), FROZEN_COPY);
}
#[test]
fn independent_inherited_owned_vectors_and_clone_domains_remain_exact() {
    let mut c = empty_claim();
    c.extensions = vec![vec![owned::raw::Extension {
        result: 0,
        operation: owned::raw::Operation::CloneString(0),
    }]];
    assert_eq!(OLD_V2.len(), 191);
    assert_eq!(OLD_V3.len(), 191);
    assert_eq!(owned::wire::encode(&c).expect("unchanged v2 bytes"), OLD_V2);
    assert_eq!(owned::wire::decode(OLD_V2).expect("v2 raw vector").claims(), &c);
    c.extensions[0][0].operation = owned::raw::Operation::CloneBorrowedString(0);
    assert_eq!(owned::wire::v3::encode(&c).expect("unchanged v3 bytes"), OLD_V3);
    assert_eq!(owned::wire::v3::decode(OLD_V3).expect("v3 raw vector").claims(), &c);
    assert!(owned::wire::encode(&c).is_err());
    assert!(owned::wire::decode(&rebrand(OLD_V3, owned::wire::HEADER, 2)).is_err());
    assert!(owned::wire::v4::decode(OLD_V2).is_err());
    assert!(owned::wire::v4::decode(OLD_V3).is_err());
    assert!(owned::wire::decode(FROZEN_COPY).is_err());
    assert!(owned::wire::v3::decode(FROZEN_COPY).is_err());
    for c in [
        owned::wire::decode(OLD_V2).expect("v2 claims"),
        owned::wire::v3::decode(OLD_V3).expect("v3 claims"),
    ] {
        let b = owned::wire::v4::encode(c.claims()).expect("v4 inherited operation");
        assert_eq!(&b[30..], if b[182] == 6 { &OLD_V2[30..] } else { &OLD_V3[30..] });
    }
}
#[test]
fn every_truncation_trailing_byte_and_unknown_tag_rejects_then_pristine_replays() {
    for end in 0..440 {
        assert!(owned::wire::v4::decode(&VECTOR[..end]).is_err(), "truncation {end}");
    }
    for (offset, value) in [(0, b'X'), (26, 3), (389, 0), (389, 10), (406, 0), (406, 10), (431, 2)]
    {
        let mut b = VECTOR.to_vec();
        b[offset] = value;
        assert!(owned::wire::v4::decode(&b).is_err(), "mutant offset {offset}");
    }
    for offset in [30, 377, 381, 394, 415, 419] {
        let mut b = VECTOR.to_vec();
        b[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(owned::wire::v4::decode(&b).is_err(), "hostile length {offset}");
    }
    let mut b = VECTOR.to_vec();
    b.push(0);
    assert!(owned::wire::v4::decode(&b).is_err());
    assert_eq!(owned::wire::v4::decode(VECTOR).expect("pristine retry").claims(), &literal_claim());
}
#[test]
fn old_encoders_decoders_and_both_field_opcodes_reject_even_after_complete_rebranding() {
    let full = literal_claim();
    for operation in [
        owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0] },
        owned::raw::Operation::CopyStructField { value: 1, ordinal: 0 },
    ] {
        let mut c = full.clone();
        c.extensions[0] = vec![owned::raw::Extension { result: 1, operation }];
        assert!(owned::wire::encode(&c).is_err());
        assert!(owned::wire::v3::encode(&c).is_err());
        let b = owned::wire::v4::encode(&c).expect("each isolated new extension");
        assert!(owned::wire::decode(&b).is_err());
        assert!(owned::wire::v3::decode(&b).is_err());
        assert!(owned::wire::decode(&rebrand(&b, owned::wire::HEADER, 2)).is_err());
        assert!(owned::wire::v3::decode(&rebrand(&b, owned::wire::v3::HEADER, 3)).is_err());
    }
    assert!(frozen::decode(VECTOR).is_err());
    assert!(owned::wire::v4::decode(FROZEN_COPY).is_err());
    // Old-domain opcode-free packets must remain old-domain packets.
    let c = empty_claim();
    for b in [
        owned::wire::encode(&c).expect("v2 empty claims"),
        owned::wire::v3::encode(&c).expect("v3 empty claims"),
    ] {
        assert!(owned::wire::v4::decode(&b).is_err());
        let mut header_only = b.clone();
        header_only[..26].copy_from_slice(owned::wire::v4::HEADER);
        assert!(owned::wire::v4::decode(&header_only).is_err());
        let mut version_only = b;
        version_only[26..30].copy_from_slice(&4u32.to_le_bytes());
        assert!(owned::wire::v4::decode(&version_only).is_err());
    }
    // v4 can physically encode inherited opcodes, but must not widen v1's frozen namespace.
    for tag in [9, 10] {
        let mut b = CORE.to_vec();
        b[277] = tag;
        assert!(frozen::decode(&b).is_err());
    }
}
