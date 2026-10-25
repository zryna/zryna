//! The shared exact source-outcome parser runs only after native host completion checks.

use zryna_abi::{ScalarOutcome, ScalarTrapCode, ScalarType};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{invocation, raw::TrapIdentity};

pub(super) const BYTES: usize = invocation::RECORD_BYTES;
#[cfg(test)]
const MAGIC: u32 = invocation::RECORD_MAGIC;
#[cfg(test)]
const END: u32 = invocation::RECORD_END;
#[cfg(test)]
const fn type_tag(result: ScalarType) -> u32 {
    invocation::scalar_tag(result)
}

pub(super) fn decode(bytes: &[u8], result: ScalarType) -> Result<ScalarOutcome, Diagnostic> {
    match invocation::decode(bytes, result).map_err(|_| rejected())? {
        invocation::Outcome::Value(value) => Ok(ScalarOutcome::Returned { value }),
        invocation::Outcome::Trap(trap) => Ok(ScalarOutcome::Trapped {
            code: match trap {
                TrapIdentity::Bounds => ScalarTrapCode::Bounds,
                TrapIdentity::Allocation => ScalarTrapCode::Allocation,
                TrapIdentity::Capacity => ScalarTrapCode::Capacity,
                TrapIdentity::Refcount => ScalarTrapCode::Refcount,
                TrapIdentity::Utf8 => ScalarTrapCode::Utf8,
            },
        }),
    }
}

pub(super) fn rejected() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4164",
        "generic native invocation produced an invalid host outcome record",
        "retain the typed source-owned invocation carrier and exact complete scalar/status frame",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zryna_abi::ScalarValue;
    fn frame(status: u32, tag: u32, bits: u32) -> Vec<u8> {
        [MAGIC, status, tag, bits, END].into_iter().flat_map(u32::to_le_bytes).collect()
    }
    #[test]
    fn exact_scalar_and_all_five_normalized_traps_only() {
        for (status, code) in [
            (1, ScalarTrapCode::Bounds),
            (2, ScalarTrapCode::Allocation),
            (3, ScalarTrapCode::Capacity),
            (4, ScalarTrapCode::Refcount),
            (5, ScalarTrapCode::Utf8),
        ] {
            for result in [ScalarType::Bool, ScalarType::I32] {
                assert_eq!(
                    decode(&frame(status, type_tag(result), 0), result)
                        .expect("frozen normalized source trap"),
                    ScalarOutcome::Trapped { code }
                );
            }
        }
        for (result, bits, value) in [
            (ScalarType::Bool, 0, ScalarValue::Bool(false)),
            (ScalarType::Bool, 1, ScalarValue::Bool(true)),
            (ScalarType::I32, u32::MAX, ScalarValue::I32(-1)),
            (ScalarType::I32, 0x8000_0000, ScalarValue::I32(i32::MIN)),
        ] {
            assert_eq!(
                decode(&frame(0, type_tag(result), bits), result).expect("canonical scalar"),
                ScalarOutcome::Returned { value }
            );
        }
    }
    #[test]
    fn malformed_stale_unknown_and_invariant_are_host_failures() {
        let pristine = frame(0, 1, 97);
        for length in 0..BYTES {
            assert!(decode(&pristine[..length], ScalarType::I32).is_err());
        }
        let mut extra = pristine.clone();
        extra.push(0);
        assert!(decode(&extra, ScalarType::I32).is_err());
        for (status, tag, bits) in [(255, 1, 0), (6, 1, 0), (1, 1, 97), (0, 0, 2), (0, 7, 0)] {
            assert!(decode(&frame(status, tag, bits), ScalarType::Bool).is_err());
            assert!(decode(&frame(status, tag, bits), ScalarType::I32).is_err());
        }
        for index in [0, 16] {
            let mut changed = pristine.clone();
            changed[index] ^= 1;
            assert!(decode(&changed, ScalarType::I32).is_err());
        }
        assert!(decode(&pristine, ScalarType::Bool).is_err());
        assert!(decode(&frame(0, 0, 2), ScalarType::Bool).is_err());
    }
}
