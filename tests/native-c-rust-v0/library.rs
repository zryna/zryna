// Foreign Rust program bytes are inert fixture data in the safe workspace.
// Only explicit outside-checkout staging may supply them to an ordinary compiler.
// This constant is not a workspace foreign implementation or an object grant.
pub(super) const FOREIGN_RUST_SOURCE: &str = r#"//! Independent Rust library fixture. Its object has not been compiled or accepted.
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

/// Full-width modulo-2^32 C-ABI addition; no allocation or foreign calls.
#[unsafe(no_mangle)]
pub extern "C" fn add(left: i32, right: i32) -> i32 {
    left.wrapping_add(right)
}

/// Sums borrowed bytes synchronously without retaining any pointer.
///
/// Valid calls provide live, aligned, writable `out` storage and read-only input
/// for `length` bytes. A null input is valid only for zero length. Inputs and
/// outputs must not overlap. Status 0 initializes `out`; status 1 means a raw
/// length above 4096 and preserves all caller storage. Status 2 is defensive
/// rejection of an invalid null caller, outside the declared valid-input domain;
/// it is not a recoverable operation status. Arbitrary nonnull pointer validity
/// is not checked or guaranteed by this function.
///
/// # Safety
/// The caller must meet those memory-validity, lifetime and nonaliasing promises.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sum_bytes(bytes: *const u8, length: usize, out: *mut i32) -> i32 {
    if out.is_null() || (length != 0 && bytes.is_null()) {
        return 2;
    }
    if length > 4096 {
        return 1;
    }
    let mut result = 0_i32;
    let mut index = 0_usize;
    while index < length {
        // The valid caller owns `length` live bytes; length is at most 4096.
        let byte = unsafe { bytes.add(index).read() };
        result = result.wrapping_add(i32::from(byte));
        index = index.wrapping_add(1);
    }
    // Writing occurs only after the complete successful read-only traversal.
    unsafe { out.write(result) };
    0
}
"#;
