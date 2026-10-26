//! Fixed pre-output transport controls, without process or executable authority.
use super::{
    Outcome, RECORD_END, RECORD_MAGIC, SourceStatus, TransportError, decode, decode_packed,
};
use crate::generic_v1::execution_v1::raw::TrapIdentity as T;
use zryna_abi::{ScalarType as Ty, ScalarValue as V};
fn record(status: u32, ty: u32, value: u32) -> Vec<u8> {
    [RECORD_MAGIC, status, ty, value, RECORD_END].into_iter().flat_map(u32::to_le_bytes).collect()
}
#[test]
fn exact_source_identity_table_and_scalar_codes_do_not_alias() {
    for (code, trap) in
        [(1, T::Bounds), (2, T::Allocation), (3, T::Capacity), (4, T::Refcount), (5, T::Utf8)]
    {
        assert_eq!(SourceStatus::Trap(trap).code(), code);
        assert_eq!(SourceStatus::from_code(code), Some(SourceStatus::Trap(trap)));
        assert_eq!(decode(&record(code, 1, 0), Ty::I32), Ok(Outcome::Trap(trap)));
        assert_eq!(
            decode(&record(0, 1, code), Ty::I32),
            Ok(Outcome::Value(V::I32(code.cast_signed())))
        );
        assert_eq!(decode_packed(u64::from(code) << 32, Ty::I32), Ok(Outcome::Trap(trap)));
    }
    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(
            decode(&record(0, 1, value.cast_unsigned()), Ty::I32),
            Ok(Outcome::Value(V::I32(value)))
        );
    }
    for (bits, value) in [(0, false), (1, true)] {
        assert_eq!(decode(&record(0, 0, bits), Ty::Bool), Ok(Outcome::Value(V::Bool(value))));
    }
}
#[test]
fn truncated_extra_stale_shape_and_every_invalid_field_are_rejected() {
    let pristine = record(0, 1, 37);
    for count in 0..20 {
        assert_eq!(decode(&pristine[..count], Ty::I32), Err(TransportError::Length));
    }
    let mut extra = pristine.clone();
    extra.push(0);
    assert_eq!(decode(&extra, Ty::I32), Err(TransportError::Length));
    for offset in [0, 16] {
        let mut changed = pristine.clone();
        changed[offset] ^= 1;
        assert_eq!(decode(&changed, Ty::I32), Err(TransportError::Framing));
    }
    assert_eq!(decode(&pristine, Ty::Bool), Err(TransportError::Type));
    assert_eq!(decode(&record(0, 2, 0), Ty::I32), Err(TransportError::Type));
    for status in [6, 254, 255, u32::MAX] {
        assert_eq!(decode(&record(status, 1, 0), Ty::I32), Err(TransportError::Status));
    }
    for status in 1..=5 {
        assert_eq!(decode(&record(status, 1, 1), Ty::I32), Err(TransportError::Scalar));
    }
    for bits in [2, u32::MAX] {
        assert_eq!(decode(&record(0, 0, bits), Ty::Bool), Err(TransportError::Scalar));
    }
    assert_eq!(decode(&pristine, Ty::I32), Ok(Outcome::Value(V::I32(37))));
}
