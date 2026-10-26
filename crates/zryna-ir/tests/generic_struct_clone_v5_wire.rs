//! Private v5 transport only: physical claims never establish source/Clone authority.
use owned::{raw, wire};
use zryna_ir::generic_v1::{Failure, owned_v2 as owned, raw as core, wire as frozen};

fn claim(operation: raw::Operation) -> raw::Program {
    raw::Program {
        graph: core::Program {
            modules: vec![],
            declarations: vec![],
            type_keys: vec![],
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            functions: vec![],
        },
        extensions: vec![vec![raw::Extension { result: 17, operation }]],
        plans: vec![],
    }
}
fn core_end(bytes: &[u8]) -> usize {
    let at = 30;
    34 + u32::from_le_bytes(bytes[at..at + 4].try_into().expect("core length")) as usize
}
fn rebrand(bytes: &[u8], header: &[u8], version: u32) -> Vec<u8> {
    assert_eq!(header.len(), 26);
    let mut bytes = bytes.to_vec();
    bytes[..26].copy_from_slice(header);
    bytes[26..30].copy_from_slice(&version.to_le_bytes());
    bytes
}
fn old_reject(bytes: &[u8]) {
    assert!(frozen::decode(bytes).is_err());
    assert!(wire::decode(bytes).is_err());
    assert!(wire::v3::decode(bytes).is_err());
    assert!(wire::v4::decode(bytes).is_err());
}

#[test]
fn old_codec_tag_ten_rejection_is_an_independent_raw_byte_control() {
    // This control uses no v5 API or raw CloneStruct variant.
    let c = claim(raw::Operation::CloneString(0x0403_0201));
    for (mut bytes, header, version) in [
        (wire::encode(&c).expect("old v2"), wire::HEADER, 2),
        (wire::v3::encode(&c).expect("old v3"), wire::v3::HEADER, 3),
        (wire::v4::encode(&c).expect("old v4"), wire::v4::HEADER, 4),
    ] {
        let opcode = core_end(&bytes) + 12;
        assert_eq!(&bytes[opcode..opcode + 5], &[6, 1, 2, 3, 4]);
        bytes[opcode] = 10;
        old_reject(&bytes);
        // Fully rewriting the domain still cannot install a new opcode in an old lane.
        old_reject(&rebrand(&bytes, header, version));
    }
}

#[test]
fn v5_is_a_distinct_opaque_carrier_and_tag_ten_is_exactly_one_value_id() {
    let _: fn(&[u8]) -> Result<wire::v5::DecodedOwnedV5, Failure> = wire::v5::decode;
    assert_eq!(wire::v5::HEADER, b"ZRYNA-GENERIC-OWNED-IR-V5\0");
    assert_eq!(wire::v5::VERSION, 5);
    assert_eq!(wire::MAX_BYTES, 32 * 1024 * 1024);
    assert_eq!(wire::MAX_CHILDREN, 1_048_576);
    for value in [0x0403_0201, u32::MAX] {
        let c = claim(raw::Operation::CloneStruct { value });
        let bytes = wire::v5::encode(&c).expect("physical v5 claims only");
        assert_eq!(&bytes[..26], wire::v5::HEADER);
        assert_eq!(&bytes[26..30], &[5, 0, 0, 0]);
        let mut expected = vec![1, 0, 0, 0, 1, 0, 0, 0, 17, 0, 0, 0, 10];
        expected.extend_from_slice(&value.to_le_bytes());
        expected.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(&bytes[core_end(&bytes)..], expected);
        assert_eq!(wire::v5::decode(&bytes).expect("immutable raw claims").claims(), &c);
        // In particular, decoding 0xffffffff does not prove a known source ID.
        old_reject(&bytes);
        assert!(wire::encode(&c).is_err());
        assert!(wire::v3::encode(&c).is_err());
        assert!(wire::v4::encode(&c).is_err());
        for (header, version) in [(wire::HEADER, 2), (wire::v3::HEADER, 3), (wire::v4::HEADER, 4)] {
            old_reject(&rebrand(&bytes, header, version));
            let mut header_only = bytes.clone();
            header_only[..26].copy_from_slice(header);
            old_reject(&header_only);
            assert!(wire::v5::decode(&header_only).is_err());
            let mut version_only = bytes.clone();
            version_only[26..30].copy_from_slice(&version.to_le_bytes());
            old_reject(&version_only);
            assert!(wire::v5::decode(&version_only).is_err());
        }
    }
}

#[test]
fn unchanged_old_messages_are_rejected_and_inherited_payloads_keep_their_literal_meanings() {
    let cases = [
        (raw::Operation::StringLiteral(vec![b'E']), vec![1, 1, 0, 0, 0, b'E'], 2),
        (raw::Operation::Move(0x0403_0201), vec![2, 1, 2, 3, 4], 2),
        (raw::Operation::Borrow { value: 0x0403_0201, exclusive: true }, vec![3, 1, 2, 3, 4, 1], 2),
        (raw::Operation::EndLoan(0x0403_0201), vec![4, 1, 2, 3, 4], 2),
        (raw::Operation::Drop(0x0403_0201), vec![5, 1, 2, 3, 4], 2),
        (raw::Operation::CloneString(0x0403_0201), vec![6, 1, 2, 3, 4], 2),
        (raw::Operation::CloneBorrowedString(0x0403_0201), vec![7, 1, 2, 3, 4], 3),
        (
            raw::Operation::StructConstruct { ty: 0x0403_0201, fields: vec![9] },
            vec![8, 1, 2, 3, 4, 1, 0, 0, 0, 9, 0, 0, 0],
            4,
        ),
        (
            raw::Operation::CopyStructField { value: 0x0403_0201, ordinal: 9 },
            vec![9, 1, 2, 3, 4, 9, 0, 0, 0],
            4,
        ),
    ];
    for (operation, payload, introduced) in cases {
        let c = claim(operation);
        let bytes = wire::v5::encode(&c).expect("v5 inherited operation");
        assert_eq!(wire::v5::decode(&bytes).expect("v5 literal meaning").claims(), &c);
        let opcode = core_end(&bytes) + 12;
        assert_eq!(&bytes[opcode..opcode + payload.len()], payload);
        for (version, old) in
            [(2, wire::encode(&c)), (3, wire::v3::encode(&c)), (4, wire::v4::encode(&c))]
        {
            if version < introduced {
                assert!(old.is_err());
                continue;
            }
            let old = old.expect("unchanged inherited old encoder");
            assert_eq!(&old[30..], &bytes[30..], "every inherited envelope byte remains exact");
            assert!(wire::v5::decode(&old).is_err());
            assert!(wire::v5::decode(&rebrand(&old, wire::v5::HEADER, version)).is_err());
            let mut version_only = old;
            version_only[26..30].copy_from_slice(&5u32.to_le_bytes());
            assert!(wire::v5::decode(&version_only).is_err());
        }
        assert!(wire::v5::decode(&frozen::encode(&c.graph).expect("frozen core v1")).is_err());
    }
}

#[test]
fn all_truncations_unknown_tags_and_trailing_bytes_reject_then_pristine_replays() {
    let c = claim(raw::Operation::CloneStruct { value: u32::MAX });
    let bytes = wire::v5::encode(&c).expect("physical v5 message");
    let opcode = core_end(&bytes) + 12;
    for end in 0..bytes.len() {
        assert!(wire::v5::decode(&bytes[..end]).is_err(), "truncation {end}");
    }
    for tag in [0, 11, 255] {
        let mut hostile = bytes.clone();
        hostile[opcode] = tag;
        assert!(wire::v5::decode(&hostile).is_err());
    }
    // Even a physically valid tag10 operand must contain all four bytes.
    for present in 0..4 {
        assert!(wire::v5::decode(&bytes[..opcode + 1 + present]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(wire::v5::decode(&trailing).is_err());
    assert_eq!(wire::v5::decode(&bytes).expect("pristine retry").claims(), &c);
}

#[test]
fn inherited_blob_vector_boolean_field_and_complete_message_limits_stay_closed() {
    let mut c = claim(raw::Operation::StringLiteral(vec![b'x'; 65_536]));
    c.plans = vec![raw::Plan {
        steps: vec![raw::Step {
            block: 0,
            position: 1,
            failure: true,
            end_loans: vec![1],
            cleanup: vec![2],
        }],
    }];
    let bytes = wire::v5::encode(&c).expect("exact inherited literal budget");
    assert_eq!(wire::v5::decode(&bytes).expect("exact literal").claims(), &c);
    let ext = core_end(&bytes);
    let plans = ext + 17 + 65_536;
    for offset in [30, ext, ext + 4, ext + 13, plans, plans + 4, plans + 17, plans + 25] {
        let mut hostile = bytes.clone();
        hostile[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(wire::v5::decode(&hostile).is_err(), "length offset {offset}");
    }
    for (offset, value) in [(ext + 17, 255), (plans + 16, 2)] {
        let mut hostile = bytes.clone();
        hostile[offset] = value;
        assert!(wire::v5::decode(&hostile).is_err());
    }
    c.extensions[0][0].operation = raw::Operation::StringLiteral(vec![b'x'; 65_537]);
    assert!(wire::v5::encode(&c).is_err());
    c.extensions[0][0].operation = raw::Operation::StructConstruct {
        ty: 0,
        fields: vec![0; zryna_layout::MAX_MEMBERS_PER_DECLARATION + 1],
    };
    assert!(wire::v5::encode(&c).is_err());
    assert!(wire::v5::decode(&vec![0; wire::MAX_BYTES + 1]).is_err());
    assert_eq!(
        wire::v5::decode(&bytes).expect("bounded pristine retry").claims(),
        &claim_with_plan()
    );
}
fn claim_with_plan() -> raw::Program {
    let mut c = claim(raw::Operation::StringLiteral(vec![b'x'; 65_536]));
    c.plans = vec![raw::Plan {
        steps: vec![raw::Step {
            block: 0,
            position: 1,
            failure: true,
            end_loans: vec![1],
            cleanup: vec![2],
        }],
    }];
    c
}
