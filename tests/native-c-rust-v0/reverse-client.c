/* Compile separately against the actual emitted pure scalar export header.
 * reverse-exports.h is future generated output, not a checked-in substitute.
 * This client does not make the Zryna export call the Rust foreign library. */
#include "reverse-exports.h"
#include <assert.h>
#include <limits.h>
#include <stdint.h>
#include <stddef.h>

int main(void) {
    for (size_t repeat = 0; repeat < 3; ++repeat) {
        assert(zryna_c_v0_e_add(20, 22) == 42);
        assert((uint32_t)zryna_c_v0_e_add(INT32_MAX, 1) == UINT32_C(2147483648));
        assert((uint32_t)zryna_c_v0_e_add(INT32_MIN, -1) == UINT32_C(2147483647));
    }
    return 0;
}
