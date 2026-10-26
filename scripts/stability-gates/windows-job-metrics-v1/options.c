#include "win.h"
BOOL wm_handle(HANDLE handle) { return handle && handle != INVALID_HANDLE_VALUE; }
void wm_error(WMContext *ctx, DWORD stage, DWORD error) {
    if (!ctx->result.stage) { ctx->result.stage = stage; ctx->result.error = error; }
}
void wm_cleanup_error(WMContext *ctx, DWORD stage, DWORD error) {
    if (!ctx->result.cleanup_stage) {
        ctx->result.cleanup_stage = stage; ctx->result.cleanup_error = error;
    }
}
BOOL wm_options(WMContext *ctx, int argc, wchar_t **argv) {
    const uint16_t *args[WM_MAX_ARGS];
    uint32_t timeout, used;
    if (argc < 7 || argc - 6 > (int)WM_MAX_ARGS ||
        wcscmp(argv[1], L"--timeout-ms") || wcscmp(argv[3], L"--output") ||
        wcscmp(argv[5], L"--") ||
        !wm_timeout((const uint16_t *)argv[2], &timeout) ||
        !wm_local_path((const uint16_t *)argv[4]) ||
        !wm_local_path((const uint16_t *)argv[6])) {
        wm_error(ctx, WM_OPTIONS, ERROR_INVALID_PARAMETER); return FALSE;
    }
    for (int i = 6; i < argc; ++i) args[i - 6] = (const uint16_t *)argv[i];
    if (!wm_quote(args, (uint32_t)(argc - 6), ctx->command, WM_COMMAND_UNITS, &used)) {
        wm_error(ctx, WM_OPTIONS, ERROR_BAD_ARGUMENTS); return FALSE;
    }
    ctx->timeout_ms = timeout; ctx->argc = (DWORD)(argc - 6); ctx->command_units = used;
    ctx->directory = argv[4]; ctx->executable = argv[6]; return TRUE;
}
static HANDLE create(const wchar_t *directory, const wchar_t *name) {
    wchar_t path[WM_COMMAND_UNITS];
    int size = _snwprintf_s(path, WM_COMMAND_UNITS, _TRUNCATE, L"%ls\\%ls", directory, name);
    if (size < 0) { SetLastError(ERROR_FILENAME_EXCED_RANGE); return INVALID_HANDLE_VALUE; }
    return CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_NEW,
                       FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN, NULL);
}
BOOL wm_outputs(WMContext *ctx) {
    DWORD attributes;
    if (!CreateDirectoryW(ctx->directory, NULL)) {
        wm_error(ctx, WM_OUTPUT, GetLastError()); return FALSE;
    }
    attributes = GetFileAttributesW(ctx->directory);
    if (attributes == INVALID_FILE_ATTRIBUTES || (attributes & FILE_ATTRIBUTE_REPARSE_POINT)) {
        wm_error(ctx, WM_OUTPUT, attributes == INVALID_FILE_ATTRIBUTES ? GetLastError() : ERROR_ACCESS_DENIED);
        return FALSE;
    }
    ctx->metadata = create(ctx->directory, L"result.json");
    if (!wm_handle(ctx->metadata)) { wm_error(ctx, WM_OUTPUT, GetLastError()); return FALSE; }
    ctx->streams[0].file = create(ctx->directory, L"stdout.raw");
    if (!wm_handle(ctx->streams[0].file)) { wm_error(ctx, WM_OUTPUT, GetLastError()); return FALSE; }
    ctx->streams[1].file = create(ctx->directory, L"stderr.raw");
    if (!wm_handle(ctx->streams[1].file)) { wm_error(ctx, WM_OUTPUT, GetLastError()); return FALSE; }
    return TRUE;
}
