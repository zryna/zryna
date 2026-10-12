/* Independent raw C oracle; compile/run and Rust object inventory are UNRUN. */
#include "library.h"
#include <assert.h>
#include <limits.h>
#include <string.h>

_Static_assert(CHAR_BIT == 8 && sizeof(int32_t) == 4, "exact i32 carrier");
_Static_assert(sizeof(size_t) == 8 && sizeof(void *) == 8, "exact LP64 tuple");

int main(void) {
    for (size_t repeat = 0; repeat < 3; ++repeat) {
        assert(add(20, 22) == 42);
        assert((uint32_t)add(INT32_MAX, 1) == UINT32_C(2147483648));
        assert((uint32_t)add(INT32_MIN, -1) == UINT32_C(2147483647));
    }
    int32_t out = 4242;
    assert(sum_bytes(NULL, 0, &out) == 0 && out == 0);
    const uint8_t small[] = {1, 2, 3};
    out = 4242;
    assert(sum_bytes(small, 3, &out) == 0 && out == 6);
    assert(small[0] == 1 && small[1] == 2 && small[2] == 3);
    uint8_t exact[4096], extra[4097], before[4097];
    memset(exact, 255, sizeof(exact));
    out = 4242;
    assert(sum_bytes(exact, sizeof(exact), &out) == 0 && out == INT32_C(1044480));
    for (size_t index = 0; index < sizeof(exact); ++index) {
        assert(exact[index] == UINT8_C(255));
    }
    memset(extra, 1, sizeof(extra));
    memcpy(before, extra, sizeof(extra));
    out = 4242;
    assert(sum_bytes(extra, sizeof(extra), &out) == 1 && out == 4242);
    assert(memcmp(extra, before, sizeof(extra)) == 0);
    /* Invalid caller controls assert defensive rejection, not ABI success or a
     * declared recoverable status. No dangling nonnull pointer is constructed. */
    out = 4242;
    assert(sum_bytes(NULL, 1, &out) == 2 && out == 4242);
    assert(sum_bytes(small, sizeof(small), NULL) == 2);
    return 0;
}
