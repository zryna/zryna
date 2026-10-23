//! Actual decoder callers must establish resource/minimum-byte credit before typed reserve.
use super::{HEADER, VERSION, raw, reader::Reader};
fn one_match_arm() -> Vec<u8> {
    // ClosedEnumMatch(type0, Place0, Value, [ordinal0, None, Edge(target0,[])])
    vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}
#[test]
fn closed_enum_match_prepays_edge_credit_before_reserving_or_visiting_an_arm() {
    let bytes = one_match_arm();
    let mut reader = Reader::new(&bytes);
    for _ in 0..8192 {
        reader.totals.edge().expect("valid checked test fixture");
    }
    assert!(reader.terminator().is_err());
    assert_eq!(reader.nonempty_reservations, 0, "exhausted edge credit reached arm reservation");
    assert_eq!(reader.elements_visited, 0, "exhausted edge credit reached an arm");
}
#[test]
fn closed_enum_match_exact_last_edge_is_charged_once_and_global_credit_precedes_reserve() {
    let bytes = one_match_arm();
    let mut reader = Reader::new(&bytes);
    for _ in 0..8191 {
        reader.totals.edge().expect("valid checked test fixture");
    }
    reader.terminator().expect("one last arm edge, without double debit");
    assert!(reader.totals.edge().is_err());
    let mut reader = Reader::new(&bytes);
    for _ in 0..16 {
        reader.totals.function();
        for _ in 0..8192 {
            reader.totals.edge().expect("valid checked test fixture");
        }
    }
    reader.totals.function();
    assert!(reader.terminator().is_err());
    assert_eq!(reader.nonempty_reservations, 0, "exhausted program edge credit reached reserve");
    assert_eq!(reader.elements_visited, 0);
}
#[test]
fn cleanup_plan_impossible_action_bytes_reject_before_typed_reserve_or_element_entry() {
    // id0 + span0 + two actions, but only two bytes remain; every action needs >=5.
    let mut bytes = vec![0; 16];
    bytes.extend_from_slice(&[2, 0, 0, 0, 0, 0]);
    let mut reader = Reader::new(&bytes);
    assert!(reader.cleanup_plan().is_err());
    assert_eq!(reader.nonempty_reservations, 0, "impossible action count reached reserve");
    assert_eq!(reader.elements_visited, 0);
}
#[test]
fn program_impossible_canonical_function_bytes_reject_before_typed_reserve() {
    let mut bytes = HEADER.to_vec();
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(raw::CONTRACT.len()).expect("valid checked test fixture").to_le_bytes(),
    );
    bytes.extend_from_slice(raw::CONTRACT.as_bytes());
    bytes.extend_from_slice(&[0; 12]); // empty modules, declarations and type keys
    bytes.extend_from_slice(&[0; 96]); // three claimed identities
    bytes.extend_from_slice(&[0; 4]); // entry0
    bytes.extend_from_slice(&[1, 0, 0, 0]); // one function, but canonical minimum is47
    bytes.extend_from_slice(&[0; 44]);
    let mut reader = Reader::new(&bytes);
    assert!(reader.program().is_err());
    assert_eq!(reader.nonempty_reservations, 0, "impossible function count reached reserve");
    assert_eq!(reader.elements_visited, 0);
}
