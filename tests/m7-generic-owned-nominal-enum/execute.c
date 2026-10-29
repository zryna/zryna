/* Source-first nominal active-payload/reverse-failure cleanup with actual allocation identities. */
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "ownership-runtime-v1.h"
#include <assert.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

extern int32_t zryna_v1_e_none(int32_t);
extern int32_t zryna_v1_e_left(int32_t);
extern int32_t zryna_v1_e_right(int32_t);
extern int32_t zryna_v1_e_nestedNone(int32_t);
extern int32_t zryna_v1_e_nestedOk(int32_t);
extern int32_t zryna_v1_e_nestedErr(int32_t);
extern int32_t zryna_v1_e_phantom(int32_t);
extern int32_t zryna_v1_e_scoreFixed(int32_t);
extern int32_t zryna_v1_e_scorePlain(int32_t);
extern int32_t zryna_v1_e_fixedText(int32_t);
extern int32_t zryna_v1_e_plainNone(int32_t);

typedef struct { uintptr_t pointer; uint64_t length; int live; } allocation_record;
static allocation_record records[16];
static size_t count, calls, fail_at, live;

static void reset(size_t failure) {
    assert(live == 0);
    count = 0;
    calls = 0;
    fail_at = failure;
}

static size_t find(const zryna_rt_o1_handle *value) {
    assert(value->capacity == value->length);
    for (size_t i = 0; i < count; i++) {
        if (records[i].pointer == value->pointer && records[i].live) {
            assert(records[i].length == value->length);
            return i;
        }
    }
    assert(!"duplicate or unavailable runtime handle");
    return 0;
}

uint32_t zryna_rt_o1_string_from_utf8_copy(
    const uint8_t *bytes, uint64_t length, zryna_rt_o1_handle *out
) {
    calls++;
    if (calls == fail_at) {
        printf("fail:%zu\n", calls);
        return 1;
    }
    assert(count < 16 && length < 65536);
    void *pointer = malloc(length ? length : 1);
    assert(pointer);
    memcpy(pointer, bytes, length);
    records[count++] = (allocation_record){(uintptr_t)pointer, length, 1};
    live++;
    printf("alloc:%zu:%.*s\n", count, (int)length, (const char *)pointer);
    *out = (zryna_rt_o1_handle){(uintptr_t)pointer, length, length};
    return 0;
}

uint32_t zryna_rt_o1_string_clone(
    const zryna_rt_o1_handle *source, zryna_rt_o1_handle *out
) {
    (void)source;
    (void)out;
    assert(!"this frozen nominal source contains no clone operation");
    return 1;
}

uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
    size_t index = find(value);
    printf("drop:%zu:%.*s\n", index + 1, (int)value->length, (const char *)value->pointer);
    memset((void *)value->pointer, 0, value->length);
    free((void *)value->pointer);
    records[index].live = 0;
    assert(live > 0);
    live--;
    return 0;
}

typedef struct {
    const char *name;
    int32_t (*function)(int32_t);
    int32_t result;
    size_t faults;
} test_case;

static const test_case cases[] = {
    {"none", zryna_v1_e_none, 10, 2},
    {"left", zryna_v1_e_left, 14, 3},
    {"right", zryna_v1_e_right, 30, 2},
    {"nestedNone", zryna_v1_e_nestedNone, 12, 2},
    {"nestedOk", zryna_v1_e_nestedOk, 24, 3},
    {"nestedErr", zryna_v1_e_nestedErr, 56, 2},
    {"phantom", zryna_v1_e_phantom, 38, 0},
    {"scoreFixed", zryna_v1_e_scoreFixed, 44, 0},
    {"scorePlain", zryna_v1_e_scorePlain, 48, 1},
    {"fixedText", zryna_v1_e_fixedText, 14, 1},
    {"plainNone", zryna_v1_e_plainNone, 46, 0},
};

static void positive(const test_case *test) {
    reset(0);
    printf("call:%s:7\n", test->name);
    assert(test->function(7) == test->result);
    assert(live == 0);
    for (size_t i = 0; i < count; i++) assert(!records[i].live);
}

static void failure(const test_case *test, size_t number) {
    reset(number);
    printf("fault:%s:7:%zu\n", test->name, number);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        (void)test->function(7);
        _exit(99);
    }
    int status;
    assert(waitpid(child, &status, 0) == child);
    assert(WIFSIGNALED(status) && WTERMSIG(status) == SIGILL);
}

int main(void) {
    const struct rlimit no_core = {0, 0};
    assert(setrlimit(RLIMIT_CORE, &no_core) == 0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
    assert(setvbuf(stdout, NULL, _IONBF, 0) == 0);
    size_t size = sizeof(cases) / sizeof(cases[0]);
    for (size_t i = 0; i < size; i++) positive(&cases[i]);
    for (size_t i = 0; i < size; i++) {
        for (size_t number = 1; number <= cases[i].faults; number++) {
            failure(&cases[i], number);
            positive(&cases[i]);
        }
    }
    return 0;
}
