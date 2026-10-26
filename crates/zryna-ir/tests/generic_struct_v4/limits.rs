//! Exact physical bounds and independently counted first-extra witnesses.
use super::claims::{empty_claim, literal_claim};
use zryna_ir::generic_v1::owned_v2 as owned;

#[test]
fn field_key_child_and_complete_message_ceilings_have_exact_first_extra_witnesses() {
    let mut c = literal_claim();
    c.extensions[0][0].operation =
        owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0; 1024] };
    let b = owned::wire::v4::encode(&c)
        .expect("1024 fields physically bounded; not a typed source seal");
    assert_eq!(owned::wire::v4::decode(&b).expect("exact field limit").claims(), &c);
    c.extensions[0][0].operation =
        owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0; 1025] };
    assert!(owned::wire::v4::encode(&c).is_err());
    let mut first_extra = b;
    first_extra[394..398].copy_from_slice(&1025u32.to_le_bytes());
    first_extra.splice(398..398, [0, 0, 0, 0]);
    assert!(owned::wire::v4::decode(&first_extra).is_err());
    // Zero is physically an untrusted vector, and is later rejected against a nonempty sealed Struct.
    c.extensions[0][0].operation = owned::raw::Operation::StructConstruct { ty: 3, fields: vec![] };
    owned::wire::v4::decode(&owned::wire::v4::encode(&c).expect("zero raw field vector"))
        .expect("raw zero vector");
    let mut c = empty_claim();
    c.graph.type_keys = vec![vec![0; 4096]];
    let b = owned::wire::v4::encode(&c).expect("4096 key bytes");
    owned::wire::v4::decode(&b).expect("exact raw key limit");
    let mut first_extra = b;
    let n = u32::from_le_bytes(first_extra[30..34].try_into().expect("core length"));
    first_extra[30..34].copy_from_slice(&(n + 1).to_le_bytes());
    first_extra[70..74].copy_from_slice(&4097u32.to_le_bytes());
    first_extra.insert(74, 0);
    assert!(owned::wire::v4::decode(&first_extra).is_err());
    c.graph.type_keys[0].push(0);
    assert!(owned::wire::v4::encode(&c).is_err());
    // 1 outer extension list + 1023 extension records + 1023*1024 field IDs = 1,048,576 children.
    let mut c = empty_claim();
    c.extensions = vec![
        (0..1023)
            .map(|result| owned::raw::Extension {
                result,
                operation: owned::raw::Operation::StructConstruct { ty: 0, fields: vec![0; 1024] },
            })
            .collect(),
    ];
    let b = owned::wire::v4::encode(&c).expect("exact independently counted child credits");
    owned::wire::v4::decode(&b).expect("exact complete child credit");
    c.plans.push(owned::raw::Plan::default());
    assert!(owned::wire::v4::encode(&c).is_err());
    let mut b = b;
    let len = b.len();
    b[len - 4..].copy_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    assert!(owned::wire::v4::decode(&b).is_err());
    // 42 envelope bytes + 136 empty-core bytes + 8183*(4+4096) + (4+3950) = 32 MiB.
    let mut c = empty_claim();
    c.graph.type_keys = vec![vec![0; 4096]; 8183];
    c.graph.type_keys.push(vec![0; 3950]);
    let b = owned::wire::v4::encode(&c).expect("exact independently counted 32 MiB");
    assert_eq!(b.len(), 32 * 1024 * 1024);
    owned::wire::v4::decode(&b).expect("exact full message");
    c.graph.type_keys.last_mut().expect("last blob").push(0);
    assert!(owned::wire::v4::encode(&c).is_err());
    let mut b = b;
    b.push(0);
    assert!(owned::wire::v4::decode(&b).is_err());
}
