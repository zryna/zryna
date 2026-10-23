/* Real Linux allocations; Rust separately spawns each controlled-fault executable. */
#include "ownership-runtime-v1.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
typedef struct { uintptr_t pointer; uint64_t length; int live; } record;
static record records[8];
static size_t count, calls, fail_at, live;

static size_t find(const zryna_rt_o1_handle *value) {
    assert(value->capacity == value->length);
    for (size_t i = 0; i < count; i++) {
        if (records[i].pointer == value->pointer && records[i].live) {
            assert(records[i].length == value->length);
            return i;
        }
    }
    assert(!"duplicate or unavailable allocation identity");
    return 0;
}
uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out) {
    calls++;
    if (calls == fail_at) { printf("fail:%zu\n", calls); return 1; }
    assert(count < 8 && length < 65536);
    void *pointer = malloc(length ? length : 1);
    assert(pointer); memcpy(pointer, bytes, length);
    records[count++] = (record){(uintptr_t)pointer, length, 1}; live++;
    printf("alloc:%zu:%.*s\n", count, (int)length, (const char *)pointer);
    *out = (zryna_rt_o1_handle){(uintptr_t)pointer, length, length};
    return 0;
}
uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source, zryna_rt_o1_handle *out) {
    (void)find(source);
    return zryna_rt_o1_string_from_utf8_copy((const uint8_t *)source->pointer, source->length, out);
}
uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
    size_t index = find(value);
    printf("drop:%zu:%.*s\n", index + 1, (int)value->length, (const char *)value->pointer);
    memset((void *)value->pointer, 0, value->length); free((void *)value->pointer);
    records[index].live = 0; assert(live > 0); live--;
    return 0;
}
int main(int argc, char **argv) {
    assert(argc == 3);
    const struct rlimit no_core = {0, 0};
    assert(setrlimit(RLIMIT_CORE, &no_core) == 0);
    assert(setvbuf(stdout, NULL, _IONBF, 0) == 0);
    fail_at = strtoul(argv[1], NULL, 10);
    size_t expected_calls = strtoul(argv[2], NULL, 10);
    assert(expected_calls <= 3 && fail_at <= expected_calls);
    int32_t value = zryna_v1_e_entry();
    assert(fail_at == 0 && value == 7 && calls == expected_calls && live == 0);
    for (size_t i = 0; i < count; i++) assert(!records[i].live);
    printf("result:%d\n", value);
    return 0;
}
