//! Malformed raw claims and byte-level allocation-credit controls.
use super::{Failure, MAX_CHILDREN, decode, encode, encoding, limits::Totals, raw, reader::Reader};
use crate::data_ownership_v1 as m3;
use zryna_source::UntrustedSpan;
fn span() -> UntrustedSpan {
    UntrustedSpan { file: 0, start: 0, end: 0 }
}
fn program() -> raw::Program {
    raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![raw::Module { id: 0, functions: 1 }],
        declarations: vec![raw::Declaration {
            module: 0,
            function: 0,
            parameters: 0,
            span: span(),
        }],
        type_keys: vec![vec![0]],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![raw::Function {
            key: vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
            original_span: span(),
            executable_export: None,
            parameters: vec![],
            result_type: raw::ValueType::Stored(0),
            places: vec![],
            loans: vec![],
            blocks: vec![raw::Block {
                id: 0,
                value_parameters: vec![],
                instructions: vec![raw::Instruction {
                    site_span: span(),
                    result: Some(raw::ValueDefinition { id: 0, ty: raw::ValueType::Stored(0) }),
                    operation: raw::Operation::BoolLiteral(true),
                }],
                terminator: raw::Terminator::Return { value: Some(0), cleanup: 0 },
            }],
            cleanup_plans: vec![raw::CleanupPlan { id: 0, span: span(), actions: vec![] }],
        }],
    }
}
fn both_reject(program: &raw::Program) {
    assert!(encode(program).is_err());
    let bytes = encoding::encode(program).expect("bounded malformed raw payload");
    assert!(decode(&bytes).is_err());
}
#[test]
fn every_unknown_tag_and_canonical_boolean_option_rejects() {
    let allowed = [
        0, 1, 2, 3, 4, 0x10, 0x11, 0x12, 0x13, 0x14, 0x20, 0x21, 0x30, 0x31, 0x32, 0x33, 0x34,
        0x35, 0x36, 0x37, 0x38, 0x40, 0x41, 0x42, 0x43, 0x44,
    ];
    for tag in 0..=u8::MAX {
        if !allowed.contains(&tag) {
            assert!(Reader::new(&[tag]).operation().is_err());
        }
    }
    for bytes in [
        &[1, 2][..],
        &[3, 9, 0, 0, 0, 0, 0, 0, 0, 0],
        &[0x10, 0, 0, 0, 0, 2],
        &[0x14, 0, 0, 0, 0, 0, 2],
        &[0x40, 0, 0, 0, 0, 2],
    ] {
        assert!(Reader::new(bytes).operation().is_err());
    }
    assert!(Reader::new(&[6]).terminator().is_err());
    assert!(Reader::new(&[2]).ty().is_err());
    assert!(Reader::new(&[2]).access().is_err());
    assert!(Reader::new(&[6]).place_kind().is_err());
    assert!(Reader::new(&[3]).static_projection().is_err());
    assert!(Reader::new(&[5]).loan_origin().is_err());
    assert!(Reader::new(&[3]).region_role().is_err());
    assert!(Reader::new(&[2]).place_or_loan().is_err());
    assert!(Reader::new(&[2]).call_argument().is_err());
    assert!(Reader::new(&[3]).match_mode().is_err());
    assert!(Reader::new(&[2]).match_binding().is_err());
    assert!(Reader::new(&[5]).trap_identity().is_err());
    assert!(Reader::new(&[3]).cleanup_action().is_err());
}
#[test]
fn malformed_contract_keys_export_and_utf8_reject_symmetrically() {
    let mut value = program();
    value.language_contract.push('x');
    both_reject(&value);
    let mut value = program();
    value.type_keys = vec![vec![0], vec![0]];
    both_reject(&value);
    let mut value = program();
    value.type_keys = vec![vec![1], vec![0]];
    both_reject(&value);
    let mut value = program();
    value.type_keys = vec![vec![0xff]];
    both_reject(&value);
    let mut value = program();
    value.functions[0].key[0] = 0x0a;
    both_reject(&value);
    let mut value = program();
    value.functions[0].executable_export = Some("é".into());
    both_reject(&value);
    let mut value = program();
    value.functions[0].executable_export = Some("a".repeat(129));
    both_reject(&value);
    let mut value = program();
    value.functions[0].blocks[0].instructions[0].operation =
        raw::Operation::StringFromUtf8 { bytes: vec![0xff], failure: 0 };
    both_reject(&value);
}
#[test]
fn nondense_or_duplicate_inventory_definition_ids_reject_symmetrically() {
    let mut value = program();
    value.modules[0].id = 1;
    both_reject(&value);
    let mut value = program();
    value.declarations[0].function = 1;
    both_reject(&value);
    let mut value = program();
    value.functions[0].blocks[0].id = 1;
    both_reject(&value);
    let mut value = program();
    value.functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("valid checked test fixture")
        .id = 1;
    both_reject(&value);
    let mut value = program();
    value.functions[0].cleanup_plans[0].id = 1;
    both_reject(&value);
    let mut value = program();
    let duplicate = value.functions[0].blocks[0].instructions[0].clone();
    value.functions[0].blocks[0].instructions.push(duplicate);
    both_reject(&value);
    let mut value = program();
    value.functions.push(value.functions[0].clone());
    both_reject(&value);
    let mut value = program();
    value.functions[0].places =
        vec![raw::Place { id: 1, ty: 0, span: span(), kind: raw::PlaceKind::Local(0) }];
    both_reject(&value);
    let mut value = program();
    value.functions[0].loans = vec![raw::Loan {
        id: 1,
        referent: 0,
        access: raw::Access::Shared,
        region: raw::Region { span: span(), role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Formal(0),
    }];
    both_reject(&value);
}
#[test]
fn child_credit_count_and_remaining_bytes_precede_reservation_or_element_visit() {
    let bytes = [1, 0, 0, 0, 0];
    let mut reader = Reader::new(&bytes);
    reader.children = MAX_CHILDREN;
    assert!(
        reader
            .plain(1, 1, |_| -> Result<(), Failure> { panic!("must reject before visit") })
            .is_err()
    );
    assert_eq!(reader.children, MAX_CHILDREN);
    let mut reader = Reader::new(&bytes);
    reader.children = MAX_CHILDREN - 1;
    assert_eq!(reader.plain(1, 1, Reader::byte).expect("valid checked test fixture"), vec![0]);
    assert_eq!(reader.children, MAX_CHILDREN);
    for bytes in [&[2, 0, 0, 0, 0][..], &[255, 255, 255, 255]] {
        let mut reader = Reader::new(bytes);
        assert!(
            reader
                .plain(MAX_CHILDREN, 4, |_| -> Result<(), Failure> {
                    panic!("no allocation/visit for impossible count")
                })
                .is_err()
        );
        assert_eq!(reader.children, 0);
    }
}
#[test]
fn inherited_limit_values_and_exact_first_extra_counters_are_pinned() {
    assert_eq!(super::MAX_BYTES, 32 * 1024 * 1024);
    assert_eq!(MAX_CHILDREN, 1_048_576);
    assert_eq!(
        (m3::MAX_MODULES, m3::MAX_FUNCTIONS_PER_MODULE, m3::MAX_FUNCTIONS_PER_PROGRAM),
        (4096, 4096, 16_384)
    );
    assert_eq!((m3::MAX_PARAMETERS_PER_FUNCTION, m3::MAX_PARAMETERS_PER_PROGRAM), (256, 262_144));
    assert_eq!(
        (m3::MAX_BLOCKS_PER_FUNCTION, m3::MAX_BLOCKS_PER_PROGRAM, m3::MAX_BLOCK_PARAMETERS),
        (4096, 65_536, 256)
    );
    assert_eq!((m3::MAX_VALUES_PER_FUNCTION, m3::MAX_VALUES_PER_PROGRAM), (16_384, 262_144));
    assert_eq!((m3::MAX_CFG_EDGES_PER_FUNCTION, m3::MAX_CFG_EDGES_PER_PROGRAM), (8_192, 131_072));
    assert_eq!((m3::MAX_CALL_EDGES, m3::MAX_AGGREGATE_OPERANDS), (65_536, 262_144));
    assert_eq!(
        (m3::MAX_PLACES_PER_FUNCTION, m3::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION),
        (65_536, 262_144)
    );
    assert_eq!(
        (
            m3::MAX_ACTIVE_BORROWS_PER_FUNCTION,
            m3::MAX_DROP_ACTIONS_PER_FUNCTION,
            m3::MAX_CLEANUP_PLANS_PER_FUNCTION
        ),
        (16_384, 262_144, 65_536)
    );
    let mut totals = Totals::default();
    totals.values(16_384).expect("valid checked test fixture");
    assert!(totals.values(1).is_err());
    let mut totals = Totals::default();
    totals.aggregate(262_144).expect("valid checked test fixture");
    assert!(totals.aggregate(1).is_err());
    let mut totals = Totals::default();
    totals.transitions(262_144).expect("valid checked test fixture");
    assert!(totals.transitions(1).is_err());
    let mut totals = Totals::default();
    totals.drops(262_144).expect("valid checked test fixture");
    assert!(totals.drops(1).is_err());
    let mut totals = Totals::default();
    for _ in 0..128 {
        totals.literal(65_536).expect("valid checked test fixture");
    }
    assert!(totals.literal(1).is_err());
    let mut totals = Totals::default();
    assert!(totals.literal(65537).is_err());
}
#[test]
fn envelope_version_and_authority_lanes_are_exact_bytes_not_seals() {
    let mut bytes = encode(&program()).expect("valid checked test fixture");
    bytes[super::HEADER.len()] = 2;
    assert!(decode(&bytes).is_err());
    let mut value = program();
    value.universe = [1; 32];
    value.linear32 = [2; 32];
    value.linux_x86_64 = [3; 32];
    let bytes = encode(&value).expect("valid checked test fixture");
    assert_eq!(decode(&bytes).expect("valid checked test fixture").claims(), &value);
}
