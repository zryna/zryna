#ifndef ZRYNA_RUST_C_V0_LIBRARY_H
#define ZRYNA_RUST_C_V0_LIBRARY_H

#include <stddef.h>
#include <stdint.h>

/* Independently authored fixture-rust-c-v0@0 draft declarations.
 * These bytes are draft material, not an accepted object or execution grant.
 * add wraps modulo 2^32. sum_bytes borrows input through return, never retains
 * pointers, and allocates nothing. Valid callers supply a live aligned nonnull
 * out slot disjoint from the input. NULL input is valid only with zero length.
 * 0 writes the byte sum; 1 means raw length >4096 and writes no output.
 * Defensive 2 for an invalid null caller is outside the declared valid-input
 * domain and must never be treated as a recoverable operation status.
 */
int32_t add(int32_t left, int32_t right);
int32_t sum_bytes(const uint8_t *bytes, size_t length, int32_t *out);

#endif
