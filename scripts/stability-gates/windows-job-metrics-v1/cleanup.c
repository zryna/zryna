#include "win.h"
static void close_owned(WMContext *ctx, HANDLE *handle) {
    if (wm_handle(*handle)) {
        if (!CloseHandle(*handle)) wm_cleanup_error(ctx, WM_CLOSE, GetLastError());
        else *handle = NULL;
    }
}
static BOOL complete_cancel(WMContext *ctx, WMStream *stream) {
    DWORD count, error;
    if (!stream->pending) return TRUE;
    if (GetOverlappedResult(stream->read, &stream->io, &count, FALSE)) {
        stream->pending = FALSE; return TRUE;
    }
    error = GetLastError();
    if (error == ERROR_IO_INCOMPLETE) return FALSE;
    stream->pending = FALSE;
    if (error != ERROR_OPERATION_ABORTED && error != ERROR_BROKEN_PIPE)
        wm_cleanup_error(ctx, WM_CANCEL, error);
    return TRUE;
}
static BOOL process_gone(WMContext *ctx) {
    DWORD waited;
    if (!ctx->result.spawned || ctx->result.child_exit_known) return TRUE;
    waited = WaitForSingleObject(ctx->process.hProcess, 0);
    if (waited == WAIT_TIMEOUT) return FALSE;
    if (waited != WAIT_OBJECT_0) {
        wm_cleanup_error(ctx, WM_WAIT, waited == WAIT_FAILED ? GetLastError() : ERROR_INVALID_DATA);
        return FALSE;
    }
    if (!GetExitCodeProcess(ctx->process.hProcess, &ctx->result.child_exit)) {
        wm_cleanup_error(ctx, WM_CHILD_EXIT, GetLastError()); return FALSE;
    }
    ctx->result.child_exit_known = TRUE; return TRUE;
}
static void drain(WMContext *ctx, ULONGLONG started) {
    for (;;) {
        DWORD active = 0, count = 0, waited;
        HANDLE waits[3];
        BOOL io_done = TRUE, main_done;
        for (unsigned int i = 0; i < 2; ++i)
            if (!complete_cancel(ctx, &ctx->streams[i])) io_done = FALSE;
        main_done = process_gone(ctx);
        if (ctx->result.spawned) {
            if (!wm_active(ctx, &active)) { wm_cleanup_error(ctx, WM_JOB_QUERY, GetLastError()); break; }
            ctx->result.job_empty = active == 0;
        } else ctx->result.job_empty = TRUE; /* No process was admitted. */
        if (io_done && main_done && ctx->result.job_empty) return;
        if (GetTickCount64() - started >= WM_CLEANUP_MS) {
            wm_cleanup_error(ctx, WM_WAIT, WAIT_TIMEOUT); return;
        }
        if (ctx->result.spawned && !main_done && wm_handle(ctx->process.hProcess))
            waits[count++] = ctx->process.hProcess;
        for (unsigned int i = 0; i < 2; ++i)
            if (ctx->streams[i].pending) waits[count++] = ctx->streams[i].event;
        ULONGLONG elapsed = GetTickCount64() - started;
        DWORD pause = elapsed >= WM_CLEANUP_MS ? 0 : (DWORD)(WM_CLEANUP_MS - elapsed);
        if (pause > 10u) pause = 10u;
        if (!count) { Sleep(pause); continue; }
        waited = WaitForMultipleObjects(count, waits, FALSE, pause);
        if (waited == WAIT_FAILED || (waited != WAIT_TIMEOUT && waited >= WAIT_OBJECT_0 + count)) {
            wm_cleanup_error(ctx, WM_WAIT, waited == WAIT_FAILED ? GetLastError() : ERROR_INVALID_DATA); return;
        }
    }
}
void wm_cleanup(WMContext *ctx) {
    ULONGLONG started = GetTickCount64();
    if (ctx->result.stage && ctx->result.spawned) {
        ctx->result.forced = TRUE;
        if (!TerminateJobObject(ctx->job, 0xe4180001u)) wm_cleanup_error(ctx, WM_TERMINATE, GetLastError());
    }
    close_owned(ctx, &ctx->streams[0].write); close_owned(ctx, &ctx->streams[1].write);
    close_owned(ctx, &ctx->input); close_owned(ctx, &ctx->process.hThread);
    for (unsigned int i = 0; i < 2; ++i) {
        WMStream *s = &ctx->streams[i];
        if (s->pending && !CancelIoEx(s->read, &s->io)) {
            DWORD error = GetLastError();
            if (error != ERROR_NOT_FOUND) wm_cleanup_error(ctx, WM_CANCEL, error);
        }
    }
    drain(ctx, started);
    /* Sole Job handle: enforce kill-on-close before artifact flush can block. */
    close_owned(ctx, &ctx->job);
    close_owned(ctx, &ctx->process.hProcess);
    for (unsigned int i = 0; i < 2; ++i) {
        WMStream *s = &ctx->streams[i];
        if (!s->pending) { close_owned(ctx, &s->read); close_owned(ctx, &s->event); }
        else wm_cleanup_error(ctx, WM_CANCEL, WAIT_TIMEOUT);
        /* Pending OVERLAPPED data/events stay in static storage until ExitProcess. */
        if (wm_handle(s->file) && !FlushFileBuffers(s->file)) wm_cleanup_error(ctx, WM_CLOSE, GetLastError());
        if (wm_handle(s->file)) {
            LARGE_INTEGER size;
            if (!GetFileSizeEx(s->file, &size)) wm_cleanup_error(ctx, WM_CLOSE, GetLastError());
            else if (size.QuadPart < 0 || (uint64_t)size.QuadPart > WM_LOG_BYTES)
                wm_cleanup_error(ctx, WM_OVERFLOW, ERROR_BUFFER_OVERFLOW);
            else if ((DWORD)size.QuadPart != s->bytes) {
                s->bytes = (DWORD)size.QuadPart;
                wm_cleanup_error(ctx, WM_WRITE, ERROR_INVALID_DATA);
            }
        }
        close_owned(ctx, &s->file);
    }
    if (GetTickCount64() - started >= WM_CLEANUP_MS)
        wm_cleanup_error(ctx, WM_WAIT, WAIT_TIMEOUT);
    if (ctx->streams[0].bytes + ctx->streams[1].bytes > WM_LOG_BYTES)
        wm_cleanup_error(ctx, WM_OVERFLOW, ERROR_BUFFER_OVERFLOW);
    ctx->result.cleanup_confirmed = !ctx->result.cleanup_stage &&
        !ctx->streams[0].pending && !ctx->streams[1].pending &&
        (!ctx->result.spawned || (ctx->result.child_exit_known && ctx->result.job_empty));
}
