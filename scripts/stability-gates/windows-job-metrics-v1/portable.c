#include "portable.h"
#include <limits.h>
static bool length(const uint16_t *s, uint32_t *out) {
    uint32_t n = 0;
    if (!s) return false;
    while (n < WM_COMMAND_UNITS && s[n]) {
        uint16_t c = s[n++];
        if (c >= 0xd800u && c <= 0xdbffu) {
            if (n == WM_COMMAND_UNITS || s[n] < 0xdc00u || s[n] > 0xdfffu) return false;
            ++n;
        } else if (c >= 0xdc00u && c <= 0xdfffu) return false;
    }
    if (n == WM_COMMAND_UNITS) return false;
    *out = n; return true;
}
static bool size_arg(const uint16_t *s, uint32_t *out) {
    uint32_t n, total = 2, slashes = 0;
    if (!length(s, &n)) return false;
    for (uint32_t i = 0; i < n; ++i) {
        if (s[i] == '\\') { ++slashes; continue; }
        total += slashes * (s[i] == '"' ? 2u : 1u) + (s[i] == '"' ? 2u : 1u);
        slashes = 0;
        if (total >= WM_COMMAND_UNITS) return false;
    }
    total += 2u * slashes;
    if (total >= WM_COMMAND_UNITS) return false;
    *out = total; return true;
}
bool wm_quote(const uint16_t *const *args, uint32_t count,
              uint16_t *out, uint32_t capacity, uint32_t *used) {
    uint32_t total = 0, at = 0;
    if (!args || !out || !used || !count || count > WM_MAX_ARGS) return false;
    for (uint32_t a = 0; a < count; ++a) {
        uint32_t part;
        if (!size_arg(args[a], &part)) return false;
        total += part + (a != 0);
        if (total >= WM_COMMAND_UNITS) return false;
    }
    if (capacity <= total) return false; /* Preflight before any destination write. */
    for (uint32_t a = 0; a < count; ++a) {
        uint32_t i = 0, slashes = 0;
        if (a) out[at++] = ' ';
        out[at++] = '"';
        while (args[a][i]) {
            uint16_t c = args[a][i++];
            if (c == '\\') { ++slashes; continue; }
            uint32_t copies = slashes * (c == '"' ? 2u : 1u) + (c == '"');
            while (copies--) out[at++] = '\\';
            out[at++] = c; slashes = 0;
        }
        slashes *= 2u;
        while (slashes--) out[at++] = '\\';
        out[at++] = '"';
    }
    out[at] = 0; *used = at; return true;
}
bool wm_timeout(const uint16_t *text, uint32_t *value) {
    uint32_t n, v = 0;
    if (!value || !length(text, &n) || !n || text[0] == '0') return false;
    for (uint32_t i = 0; i < n; ++i) {
        if (text[i] < '0' || text[i] > '9') return false;
        uint32_t digit = text[i] - '0';
        if (v > (WM_MAX_TIMEOUT_MS - digit) / 10u) return false;
        v = v * 10u + digit;
    }
    if (!v || v > WM_MAX_TIMEOUT_MS) return false;
    *value = v; return true;
}
bool wm_local_path(const uint16_t *text) {
    uint32_t n, component = 3;
    if (!length(text, &n) || n < 4 || n > WM_COMMAND_UNITS - 32u) return false;
    if (!((text[0] >= 'A' && text[0] <= 'Z') || (text[0] >= 'a' && text[0] <= 'z')) ||
        text[1] != ':' || text[2] != '\\') return false;
    for (uint32_t i = 3; i <= n; ++i) {
        uint16_t c = text[i];
        if (!c || c == '\\') {
            uint32_t size = i - component;
            if (!size || text[i - 1u] == '.' || text[i - 1u] == ' ') return false;
            component = i + 1u;
        } else if (c < 32 || c == '/' || c == ':' || c == '"' || c == '<' ||
                   c == '>' || c == '|' || c == '?' || c == '*') return false;
    }
    return true;
}
bool wm_add64(uint64_t a, uint64_t b, uint64_t *out) {
    if (!out || a > UINT64_MAX - b) return false;
    *out = a + b; return true;
}
bool wm_deadline(uint64_t start, uint64_t frequency, uint32_t ms, uint64_t *out) {
    uint64_t whole, fraction, delta, result;
    if (!out || !frequency || frequency > INT64_MAX || !ms || ms > WM_MAX_TIMEOUT_MS ||
        start > INT64_MAX || frequency / 1000u > UINT64_MAX / ms) return false;
    whole = (frequency / 1000u) * ms;
    fraction = ((frequency % 1000u) * ms + 999u) / 1000u;
    if (!wm_add64(whole, fraction, &delta) || !wm_add64(start, delta, &result) || result > INT64_MAX)
        return false;
    *out = result; return true;
}
bool wm_remaining(uint64_t now, uint64_t deadline, uint64_t frequency, uint32_t *out) {
    uint64_t delta, whole, remainder, rounded;
    if (!out || !frequency) return false;
    if (now >= deadline) { *out = 0; return true; }
    delta = deadline - now; whole = delta / frequency; remainder = delta % frequency;
    if (whole > UINT32_MAX / 1000u || remainder > (UINT64_MAX - frequency + 1u) / 1000u)
        return false;
    rounded = whole * 1000u + (remainder * 1000u + frequency - 1u) / frequency;
    if (rounded > UINT32_MAX) return false;
    *out = (uint32_t)rounded; return true;
}
bool wm_reserve(uint32_t total, uint32_t count, uint32_t *out) {
    if (!out || total > WM_LOG_BYTES || count > WM_LOG_BYTES - total) return false;
    *out = total + count; return true;
}
