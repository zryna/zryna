//! Exact/first-extra primitive and global counters, independent of source admissibility.
use super::{Failure, MAX_BYTES, MAX_CHILDREN, limits::Totals, reader::Reader, writer::Writer};
#[test]
fn writer_byte_and_cumulative_child_first_extra_are_atomic() {
    let mut writer = Writer::new();
    let chunk = vec![0; 1024 * 1024];
    for _ in 0..32 {
        writer.bytes(&chunk).expect("valid checked test fixture");
    }
    assert_eq!(writer.bytes.len(), MAX_BYTES);
    assert!(writer.byte(0).is_err());
    assert_eq!(writer.bytes.len(), MAX_BYTES);
    let mut writer = Writer::new();
    writer.count(MAX_CHILDREN).expect("valid checked test fixture");
    let before = writer.bytes.clone();
    assert!(writer.count(1).is_err());
    assert_eq!(writer.bytes, before);
}
#[test]
fn global_parameters_blocks_values_edges_calls_and_instances_use_existing_caps() {
    let mut totals = Totals::default();
    for _ in 0..1024 {
        totals.parameters(256).expect("valid checked test fixture");
    }
    assert!(totals.parameters(1).is_err());
    let mut totals = Totals::default();
    for _ in 0..16 {
        totals.blocks(4096).expect("valid checked test fixture");
    }
    assert!(totals.blocks(1).is_err());
    let mut totals = Totals::default();
    for _ in 0..16 {
        totals.function();
        totals.values(16384).expect("valid checked test fixture");
    }
    totals.function();
    assert!(totals.values(1).is_err());
    let mut totals = Totals::default();
    for _ in 0..16 {
        totals.function();
        for _ in 0..8192 {
            totals.edge().expect("valid checked test fixture");
        }
    }
    totals.function();
    assert!(totals.edge().is_err());
    let mut totals = Totals::default();
    for _ in 0..65536 {
        totals.call().expect("valid checked test fixture");
    }
    assert!(totals.call().is_err());
    let mut totals = Totals::default();
    for _ in 0..4096 {
        totals.generic_function().expect("valid checked test fixture");
    }
    assert!(totals.generic_function().is_err());
}
#[test]
fn inherited_category_credit_rejects_before_vector_reservation_or_visit() {
    let bytes = [1, 0, 0, 0, 0];
    let mut reader = Reader::new(&bytes);
    reader.totals.values(16384).expect("valid checked test fixture");
    assert!(
        reader
            .vector(256, 1, Totals::values, |_| -> Result<(), Failure> {
                panic!("category credit must precede allocation/visit")
            })
            .is_err()
    );
    assert_eq!(reader.children, 0);
    let impossible = [1, 1, 0, 0]; // 257, one above a 256-child field.
    let mut reader = Reader::new(&impossible);
    assert!(
        reader
            .plain(256, 0, |_| -> Result<(), Failure> {
                panic!("field count must precede allocation/visit")
            })
            .is_err()
    );
    assert_eq!(reader.children, 0);
}
