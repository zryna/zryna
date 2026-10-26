#include "../portable.h"
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
static uint16_t text[WM_COMMAND_UNITS], expected[WM_COMMAND_UNITS], actual[WM_COMMAND_UNITS];
static const uint16_t *ascii(const char *in, uint16_t *out) {
    size_t i = 0;
    do { out[i] = (unsigned char)in[i]; } while (in[i++] != 0);
    return out;
}
static void vector(const char *in, const char *want) {
    const uint16_t *args[] = {ascii(in, text)};
    uint32_t used = UINT32_MAX;
    ascii(want, expected);
    assert(wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    assert(used == strlen(want));
    assert(memcmp(actual, expected, (used + 1u) * sizeof(uint16_t)) == 0);
    actual[0] = 0x1234u; used = UINT32_MAX;
    assert(!wm_quote(args, 1, actual, (uint32_t)strlen(want), &used));
    assert(actual[0] == 0x1234u && used == UINT32_MAX);
}
static void quotes(void) {
    vector("", "\"\""); vector("abc", "\"abc\"");
    vector("a b", "\"a b\""); vector("a\tb", "\"a\tb\"");
    vector("a\"b", "\"a\\\"b\"");
    vector("C:\\x\\", "\"C:\\x\\\\\"");
    vector("x\\\"y", "\"x\\\\\\\"y\"");
    vector("\\\\", "\"\\\\\\\\\"");
    puts("ok 1 - eight independent quoting vectors and exact missing-NUL boundaries");
}
static void unicode_and_limits(void) {
    const uint16_t unicode[] = {0x03a9u, 0xd83du, 0xde00u, 0};
    const uint16_t want[] = {'"', 0x03a9u, 0xd83du, 0xde00u, '"', 0};
    const uint16_t bad1[] = {0xd800u, 0}, bad2[] = {0xdc00u, 0};
    const uint16_t *args[] = {unicode}; uint32_t used = 0;
    assert(wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    assert(used == 5 && memcmp(actual, want, sizeof(want)) == 0);
    args[0] = bad1; assert(!wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    args[0] = bad2; assert(!wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    assert(!wm_quote(args, 0, actual, WM_COMMAND_UNITS, &used));
    assert(!wm_quote(args, WM_MAX_ARGS + 1u, actual, WM_COMMAND_UNITS, &used));
    for (uint32_t i = 0; i < WM_COMMAND_UNITS - 3u; ++i) text[i] = 'x';
    text[WM_COMMAND_UNITS - 3u] = 0; args[0] = text;
    assert(wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    assert(used == WM_COMMAND_UNITS - 1u);
    text[WM_COMMAND_UNITS - 3u] = 'x'; text[WM_COMMAND_UNITS - 2u] = 0;
    assert(!wm_quote(args, 1, actual, WM_COMMAND_UNITS, &used));
    puts("ok 2 - UTF16, argc, exact32767 and first-extra command bounds");
}
static void multiple_and_paths(void) {
    uint16_t first[32], second[32]; uint32_t used = 0;
    const uint16_t *args[] = {ascii("C:\\Program Files\\t.exe", first), ascii("", second)};
    ascii("\"C:\\Program Files\\t.exe\" \"\"", expected);
    assert(wm_quote(args, 2, actual, WM_COMMAND_UNITS, &used));
    assert(memcmp(actual, expected, (used + 1u) * sizeof(uint16_t)) == 0);
    assert(wm_local_path(ascii("C:\\private\\t.exe", text)));
    assert(wm_local_path(ascii("d:\\private space\\out", text)));
    const char *bad[] = {"relative", "C:relative", "\\\\server\\share", "C:\\..\\x", "C:\\x:ads", "C:\\x.", "C:\\x ", "C:/x", "C:\\\\x"};
    for (size_t i = 0; i < sizeof(bad)/sizeof(bad[0]); ++i)
        assert(!wm_local_path(ascii(bad[i], text)));
    puts("ok 3 - separated argv and explicit local path scope");
}
static void numbers(void) {
    uint32_t value = 71; uint64_t wide = 71;
    assert(wm_timeout(ascii("1", text), &value) && value == 1);
    assert(wm_timeout(ascii("900000", text), &value) && value == WM_MAX_TIMEOUT_MS);
    const char *bad[] = {"", "0", "01", "+1", "1 ", "900001", "4294967296"};
    for (size_t i = 0; i < sizeof(bad)/sizeof(bad[0]); ++i) {
        value = 71; assert(!wm_timeout(ascii(bad[i], text), &value) && value == 71);
    }
    assert(wm_add64(UINT64_MAX - 1u, 1, &wide) && wide == UINT64_MAX);
    wide = 71; assert(!wm_add64(UINT64_MAX, 1, &wide) && wide == 71);
    assert(wm_deadline(500, 10000000, 100, &wide) && wide == 1000500);
    assert(wm_deadline(0, 3, 1, &wide) && wide == 1);
    assert(wm_remaining(500, 1000500, 10000000, &value) && value == 100);
    assert(wm_remaining(1000500, 1000500, 10000000, &value) && value == 0);
    assert(wm_remaining(0, 1, 3, &value) && value == 334);
    wide = 71; assert(!wm_deadline(UINT64_MAX, 10000000, 1, &wide) && wide == 71);
    assert(!wm_deadline(0, 0, 1, &wide));
    assert(!wm_remaining(0, 1, 0, &value));
    puts("ok 4 - canonical timeout and exact checked QPC arithmetic");
}
static void quota(void) {
    uint32_t out = 71;
    assert(wm_reserve(WM_LOG_BYTES - 1u, 1, &out) && out == WM_LOG_BYTES);
    out = 71; assert(!wm_reserve(WM_LOG_BYTES, 1, &out) && out == 71);
    assert(!wm_reserve(UINT32_MAX, 1, &out) && out == 71);
    assert(wm_reserve(0, 0, &out) && out == 0);
    puts("ok 5 - combined raw exact16MiB first-extra and arithmetic overflow");
}
int main(void) {
    puts("TAP version 13"); quotes(); unicode_and_limits(); multiple_and_paths(); numbers(); quota();
    puts("1..5"); return 0;
}
