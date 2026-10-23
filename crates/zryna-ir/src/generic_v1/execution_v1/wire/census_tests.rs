//! The exact serializer must census bounded bytes/children before any structural preflight.
use super::{MAX_BYTES, MAX_CHILDREN, encode_checked, encoding, raw, writer::Writer};
use zryna_source::UntrustedSpan;
#[test]
fn count_only_census_rejects_first_extra_child_before_visiting_it() {
    let mut writer = Writer::counter();
    writer.count(MAX_CHILDREN).expect("valid checked test fixture");
    let before = writer.length();
    assert!(writer.vector(&[()], |_, ()| panic!("first extra child was visited")).is_err());
    assert_eq!(writer.length(), before);
    assert!(writer.bytes.is_empty());
    assert_eq!(writer.bytes.capacity(), 0);
}
#[test]
fn count_only_census_exact_byte_ceiling_and_first_extra_allocate_nothing() {
    let mut writer = Writer::counter();
    let chunk = [0; 1024];
    for _ in 0..(MAX_BYTES / chunk.len()) {
        writer.bytes(&chunk).expect("valid checked test fixture");
    }
    assert_eq!(writer.length(), MAX_BYTES);
    assert!(writer.byte(0).is_err());
    assert_eq!(writer.length(), MAX_BYTES);
    assert!(writer.bytes.is_empty());
    assert_eq!(writer.bytes.capacity(), 0);
}
#[test]
fn real_over_quota_program_never_enters_preflight_or_its_index_reservation() {
    let span = UntrustedSpan { file: 0, start: 0, end: 0 };
    let program = raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: vec![],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: vec![raw::Function {
            key: vec![],
            original_span: span,
            executable_export: None,
            parameters: vec![],
            result_type: raw::ValueType::Unit,
            places: vec![],
            loans: vec![],
            blocks: vec![],
            cleanup_plans: vec![raw::CleanupPlan {
                id: 0,
                span,
                actions: vec![raw::CleanupAction::DropPlace(0); MAX_CHILDREN],
            }],
        }],
    };
    assert!(
        encode_checked(&program, |_| panic!(
            "over-quota census entered preflight/index allocation"
        ))
        .is_err()
    );
}
#[test]
fn same_serializer_counter_and_output_have_identical_complete_lengths() {
    let program = raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: vec![],
        universe: [1; 32],
        linear32: [2; 32],
        linux_x86_64: [3; 32],
        entry_module: 0,
        functions: vec![],
    };
    let bytes = encoding::encode(&program).expect("valid checked test fixture");
    assert_eq!(encoding::census(&program).expect("valid checked test fixture"), bytes.len());
}
