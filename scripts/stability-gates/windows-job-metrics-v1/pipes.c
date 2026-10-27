#include "win.h"
static BOOL make_pipe(WMContext *ctx, WMStream *stream, unsigned int ordinal) {
    SECURITY_ATTRIBUTES inherited = {(DWORD)sizeof(SECURITY_ATTRIBUTES), NULL, TRUE};
    wchar_t name[128]; uint64_t stamp; DWORD error, transferred;
    if (!wm_clock(&stamp)) { wm_error(ctx, WM_CLOCK, GetLastError()); return FALSE; }
    if (_snwprintf_s(name, 128, _TRUNCATE, L"\\\\.\\pipe\\zryna-m418-%lu-%llu-%u",
                    GetCurrentProcessId(), (unsigned long long)stamp, ordinal) < 0) {
        wm_error(ctx, WM_PIPE, ERROR_INVALID_NAME); return FALSE;
    }
    stream->event = CreateEventW(NULL, TRUE, FALSE, NULL);
    if (!wm_handle(stream->event)) { wm_error(ctx, WM_PIPE, GetLastError()); return FALSE; }
    stream->io.hEvent = stream->event;
    stream->read = CreateNamedPipeW(name,
        PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
        1, 0, (DWORD)sizeof(stream->buffer), 0, NULL);
    if (!wm_handle(stream->read)) { wm_error(ctx, WM_PIPE, GetLastError()); return FALSE; }
    if (!ConnectNamedPipe(stream->read, &stream->io)) {
        error = GetLastError();
        if (error == ERROR_IO_PENDING) stream->pending = TRUE;
        else if (error != ERROR_PIPE_CONNECTED) { wm_error(ctx, WM_PIPE, error); return FALSE; }
    }
    stream->write = CreateFileW(name, GENERIC_WRITE, 0, &inherited, OPEN_EXISTING,
                               FILE_ATTRIBUTE_NORMAL | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, NULL);
    if (!wm_handle(stream->write)) { wm_error(ctx, WM_PIPE, GetLastError()); return FALSE; }
    if (stream->pending) {
        DWORD waited = WaitForSingleObject(stream->event, WM_CLEANUP_MS);
        if (waited != WAIT_OBJECT_0) {
            wm_error(ctx, WM_PIPE, waited == WAIT_FAILED ? GetLastError() : WAIT_TIMEOUT); return FALSE;
        }
        if (!GetOverlappedResult(stream->read, &stream->io, &transferred, FALSE)) {
            wm_error(ctx, WM_PIPE, GetLastError()); return FALSE;
        }
        stream->pending = FALSE;
    }
    return TRUE;
}
BOOL wm_pipes(WMContext *ctx) {
    SECURITY_ATTRIBUTES inherited = {(DWORD)sizeof(SECURITY_ATTRIBUTES), NULL, TRUE};
    ctx->input = CreateFileW(L"NUL", GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
                             &inherited, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (!wm_handle(ctx->input)) { wm_error(ctx, WM_PIPE, GetLastError()); return FALSE; }
    return make_pipe(ctx, &ctx->streams[0], 0) && make_pipe(ctx, &ctx->streams[1], 1);
}
