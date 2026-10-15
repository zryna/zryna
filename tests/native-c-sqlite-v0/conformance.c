/* Direct adapter oracle. This is separate from generated Zryna wrapper execution. */
#include "library.h"
#include "sqlite3.h"
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>

_Static_assert(sizeof(void *) == 8 && sizeof(size_t) == 8 && sizeof(int) == 4,
               "Linux x86-64 LP64 fixture");
_Static_assert(_Generic(&sqlite_fixture_copy_bytes,
    int32_t (*)(const uint8_t *, size_t, uint8_t **, size_t *): 1, default: 0),
    "exact adapter copy ABI");
_Static_assert(_Generic(&sqlite_fixture_release_bytes,
    void (*)(uint8_t *): 1, default: 0), "exact adapter release ABI");

void *__real_sqlite3_malloc(int);
void __real_sqlite3_free(void *);
static size_t attempts, allocated, released, live;
static int inject_allocation_failure;
static void *owner;

void *__wrap_sqlite3_malloc(int count) {
    ++attempts;
    assert(count > 0 && count <= 4096);
    if (inject_allocation_failure) return NULL;
    assert(live == 0);
    void *value = __real_sqlite3_malloc(count);
    assert(value != NULL); /* Real OOM is not the injected failure case. */
    ++allocated;
    ++live;
    owner = value;
    return value;
}

void __wrap_sqlite3_free(void *value) {
    assert(value != NULL && value == owner && live == 1);
    __real_sqlite3_free(value);
    ++released;
    --live;
    owner = NULL;
}

int main(void) {
    const size_t lengths[] = {0, 3, 4096};
    uint8_t input[4097], before[4097];
    for (size_t i = 0; i < sizeof(input); ++i) input[i] = (uint8_t)i;
    memcpy(before, input, sizeof(input));
    for (size_t test = 0; test < 3; ++test) {
        const size_t length = lengths[test];
        uint8_t *output = (uint8_t *)(uintptr_t)1;
        size_t count = 99, prior = attempts;
        assert(sqlite_fixture_copy_bytes(length ? input : NULL, length, &output, &count) == 0);
        assert(count == length && memcmp(input, before, sizeof(input)) == 0);
        if (length == 0) {
            assert(output == NULL && attempts == prior && live == 0);
        } else {
            assert(output != input && output == owner && live == 1);
            assert(memcmp(output, input, length) == 0 && attempts == prior + 1);
            sqlite_fixture_release_bytes(output);
            assert(live == 0 && allocated == released);
        }
    }
    uint8_t *output = (uint8_t *)(uintptr_t)1;
    size_t count = 99, prior = attempts;
    assert(sqlite_fixture_copy_bytes(input, sizeof(input), &output, &count) == 1);
    assert(output == (uint8_t *)(uintptr_t)1 && count == 99 && attempts == prior);
    assert(memcmp(input, before, sizeof(input)) == 0);
    assert(sqlite_fixture_copy_bytes(NULL, 3, &output, &count) == 2);
    assert(output == (uint8_t *)(uintptr_t)1 && count == 99 && attempts == prior);
    assert(sqlite_fixture_copy_bytes(input, 3, NULL, &count) == 2);
    assert(count == 99 && attempts == prior);
    assert(sqlite_fixture_copy_bytes(input, 3, &output, NULL) == 2);
    assert(output == (uint8_t *)(uintptr_t)1 && attempts == prior);
    inject_allocation_failure = 1;
    assert(sqlite_fixture_copy_bytes(input, 3, &output, &count) == 1);
    assert(output == (uint8_t *)(uintptr_t)1 && count == 99 && live == 0);
    assert(memcmp(input, before, sizeof(input)) == 0);
    assert(allocated == released && attempts == prior + 1);
    inject_allocation_failure = 0;
    assert(sqlite_fixture_copy_bytes(input, 3, &output, &count) == 0);
    assert(live == 1 && count == 3 && memcmp(output, input, 3) == 0);
    sqlite_fixture_release_bytes(output);
    assert(live == 0 && allocated == released);
    assert(memcmp(input, before, sizeof(input)) == 0);
    puts("sqlite adapter boundaries, injected failure, recovery and exactly-once release: ok");
    return 0;
}
