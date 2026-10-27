#ifndef WM_PORTABLE_H
#define WM_PORTABLE_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define WM_MAX_ARGS 128u
#define WM_COMMAND_UNITS 32767u
#define WM_LOG_BYTES (16u * 1024u * 1024u)
#define WM_MAX_TIMEOUT_MS 900000u
#define WM_CLEANUP_MS 10000u
bool wm_quote(const uint16_t *const *args, uint32_t count,
              uint16_t *out, uint32_t capacity, uint32_t *used);
bool wm_timeout(const uint16_t *text, uint32_t *value);
bool wm_local_path(const uint16_t *text);
bool wm_add64(uint64_t a, uint64_t b, uint64_t *out);
bool wm_deadline(uint64_t start, uint64_t frequency, uint32_t ms, uint64_t *out);
bool wm_remaining(uint64_t now, uint64_t deadline, uint64_t frequency, uint32_t *out);
bool wm_reserve(uint32_t total, uint32_t count, uint32_t *out);
#endif
