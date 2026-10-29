/* Four frozen allocation roles, bounded records, exact identity and one release each. */
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "ownership-runtime-v1.h"
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
typedef struct { uintptr_t pointer; uint64_t length; unsigned releases; int live; } record;
static record records[8];
static size_t count, calls, fail_at, live, expected_calls;
static FILE *identities;

static size_t find(const zryna_rt_o1_handle *value) {
    assert(value->capacity == value->length);
    for (size_t i = 0; i < count; i++) {
        if (records[i].pointer == value->pointer && records[i].live) {
            assert(records[i].length == value->length && records[i].releases == 0);
            return i;
        }
    }
    assert(!"duplicate or unavailable allocation identity");
    return 0;
}
uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out) {
    calls++; assert(calls <= expected_calls);
    if (calls == fail_at) {
        printf("fail:%zu\n", calls);
        assert(fprintf(identities, "{\"operation\":\"failure\",\"call\":%zu}\n", calls) > 0);
        return 1;
    }
    assert(count < sizeof(records) / sizeof(records[0]) && length < 65536);
    void *pointer = malloc(length ? (size_t)length : 1);
    assert(pointer);
    /* These fixtures keep every prior allocation alive until allocation completes. */
    for (size_t i = 0; i < count; i++) assert(records[i].pointer != (uintptr_t)pointer);
    memcpy(pointer, bytes, (size_t)length);
    records[count++] = (record){(uintptr_t)pointer, length, 0, 1}; live++;
    printf("alloc:%zu:%.*s\n", count, (int)length, (const char *)pointer);
    assert(fprintf(identities, "{\"operation\":\"allocate\",\"role\":%zu,\"pointer\":\"%" PRIxPTR "\",\"length\":%" PRIu64 "}\n",
        count, (uintptr_t)pointer, length) > 0);
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
    assert(fprintf(identities, "{\"operation\":\"release\",\"role\":%zu,\"pointer\":\"%" PRIxPTR "\",\"length\":%" PRIu64 "}\n",
        index + 1, value->pointer, value->length) > 0);
    memset((void *)value->pointer, 0, (size_t)value->length); free((void *)value->pointer);
    records[index].live = 0; records[index].releases++; assert(live > 0); live--;
    return 0;
}
static size_t parse_count(const char *text) {
    char *end = NULL; errno = 0;
    unsigned long value = strtoul(text, &end, 10);
    assert(errno == 0 && end != text && *end == '\0' && value <= 4);
    return (size_t)value;
}
int main(int argc, char **argv) {
    assert(argc == 4);
    const struct rlimit no_core = {0, 0};
    assert(setrlimit(RLIMIT_CORE, &no_core) == 0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
    assert(setvbuf(stdout, NULL, _IONBF, 0) == 0);
    identities = fopen(argv[3], "wx"); assert(identities);
    assert(setvbuf(identities, NULL, _IONBF, 0) == 0);
    fail_at = parse_count(argv[1]); expected_calls = parse_count(argv[2]);
    assert(fail_at <= expected_calls);
    int32_t value = zryna_v1_e_entry();
    /* Any fault returning here fails; Rust requires SIGILL and exact cleanup stdout. */
    assert(fail_at == 0 && value == 7 && calls == expected_calls && live == 0);
    assert(count == expected_calls);
    for (size_t i = 0; i < count; i++) assert(!records[i].live && records[i].releases == 1);
    assert(fclose(identities) == 0);
    printf("result:%d\n", value);
    return 0;
}
