#include "win.h"
static BOOL append(WMContext *ctx, WMStream *stream, DWORD count) {
    uint32_t reserved;
    DWORD at = 0;
    if (count > (DWORD)sizeof(stream->buffer) ||
        !wm_reserve(ctx->streams[0].bytes + ctx->streams[1].bytes, count, &reserved)) {
        wm_error(ctx, WM_OVERFLOW, ERROR_BUFFER_OVERFLOW); return FALSE;
    }
    while (at < count) {
        DWORD written = 0;
        if (!WriteFile(stream->file, stream->buffer + at, count - at, &written, NULL)) {
            wm_error(ctx, WM_WRITE, GetLastError()); return FALSE;
        }
        if (!written || written > count - at) {
            wm_error(ctx, WM_WRITE, ERROR_WRITE_FAULT); return FALSE;
        }
        at += written; stream->bytes += written;
    }
    return TRUE;
}
BOOL wm_complete(WMContext *ctx, WMStream *stream) {
    DWORD count = 0, error;
    if (!GetOverlappedResult(stream->read, &stream->io, &count, FALSE)) {
        error = GetLastError();
        if (error == ERROR_IO_INCOMPLETE) return TRUE;
        stream->pending = FALSE;
        if (error == ERROR_BROKEN_PIPE) { stream->eof = TRUE; return TRUE; }
        wm_error(ctx, WM_READ, error); return FALSE;
    }
    stream->pending = FALSE;
    if (!count) { stream->eof = TRUE; return TRUE; }
    return append(ctx, stream, count);
}
BOOL wm_arm(WMContext *ctx, WMStream *stream) {
    DWORD error;
    if (stream->eof || stream->pending) return TRUE;
    ZeroMemory(&stream->io, sizeof(stream->io)); stream->io.hEvent = stream->event;
    if (!ResetEvent(stream->event)) { wm_error(ctx, WM_READ, GetLastError()); return FALSE; }
    if (!ReadFile(stream->read, stream->buffer, (DWORD)sizeof(stream->buffer), NULL, &stream->io)) {
        error = GetLastError();
        if (error == ERROR_IO_PENDING) { stream->pending = TRUE; return TRUE; }
        if (error == ERROR_BROKEN_PIPE) { stream->eof = TRUE; return TRUE; }
        wm_error(ctx, WM_READ, error); return FALSE;
    }
    stream->pending = TRUE;
    return wm_complete(ctx, stream);
}
