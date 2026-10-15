#ifndef ZRYNA_SQLITE_C_V0_LIBRARY_H
#define ZRYNA_SQLITE_C_V0_LIBRARY_H

#include <limits.h>
#include <stddef.h>
#include <stdint.h>

_Static_assert(CHAR_BIT == 8, "eight-bit bytes");
_Static_assert(sizeof(int32_t) == 4 && _Alignof(int32_t) == 4, "exact i32");
_Static_assert(sizeof(int) == 4 && _Alignof(int) == 4, "SQLite int carrier");
_Static_assert(INT_MIN == INT32_MIN && INT_MAX == INT32_MAX, "signed int range");
_Static_assert(sizeof(size_t) == 8 && _Alignof(size_t) == 8, "LP64 count");
_Static_assert((size_t)-1 > 0, "unsigned size_t");
_Static_assert(sizeof(void *) == 8 && _Alignof(void *) == 8, "LP64 pointer");

/* fixture-sqlite-c-v0@0, synchronous byte copy only. The caller supplies live,
 * aligned, separate output slots, disjoint from the input; bytes remain stable
 * through return. NULL input is admitted only at length zero. Status 0 writes
 * a fresh SQLite-owned allocation and exact count, or (NULL, 0) without any
 * allocation. Status 1 (length >4096 or allocation failure) writes neither
 * output. Defensive status 2 rejects invalid null callers before allocation
 * or writes; it is outside the declared valid-input operation status domain.
 * Every nonnull success pointer is the live sqlite3_malloc allocation itself;
 * no interior pointer or private Zryna allocation is returned. Release consumes
 * that exact live pointer once via sqlite3_free after all reads/copies finish.
 * No pointer retention, callbacks, reentry, concurrency or unwind is admitted.
 */
int32_t sqlite_fixture_copy_bytes(const uint8_t *bytes, size_t length,
                                  uint8_t **out_bytes, size_t *out_length);
void sqlite_fixture_release_bytes(uint8_t *bytes);

#endif
