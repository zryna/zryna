#include "win.h"
BOOL wm_clock(uint64_t *value) {
    LARGE_INTEGER counter;
    if (!QueryPerformanceCounter(&counter)) return FALSE;
    if (counter.QuadPart < 0) { SetLastError(ERROR_INVALID_DATA); return FALSE; }
    *value = (uint64_t)counter.QuadPart; return TRUE;
}
BOOL wm_active(WMContext *ctx, DWORD *active) {
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION info;
    DWORD bytes = 0;
    ZeroMemory(&info, sizeof(info));
    if (!QueryInformationJobObject(ctx->job, JobObjectBasicAccountingInformation,
                                   &info, (DWORD)sizeof(info), &bytes)) return FALSE;
    if (bytes != sizeof(info)) { SetLastError(ERROR_INVALID_DATA); return FALSE; }
    *active = info.ActiveProcesses; return TRUE;
}
static BOOL main_exit(WMContext *ctx) {
    DWORD waited;
    if (ctx->result.child_exit_known) return TRUE;
    waited = WaitForSingleObject(ctx->process.hProcess, 0);
    if (waited == WAIT_TIMEOUT) return TRUE;
    if (waited != WAIT_OBJECT_0) {
        wm_error(ctx, WM_WAIT, waited == WAIT_FAILED ? GetLastError() : ERROR_INVALID_DATA); return FALSE;
    }
    if (!GetExitCodeProcess(ctx->process.hProcess, &ctx->result.child_exit)) {
        wm_error(ctx, WM_CHILD_EXIT, GetLastError()); return FALSE;
    }
    ctx->result.child_exit_known = TRUE;
    if (ctx->result.child_exit) { wm_error(ctx, WM_CHILD_EXIT, 0); return FALSE; }
    return TRUE;
}
static BOOL budget(WMContext *ctx, uint32_t *remaining) {
    uint64_t now;
    if (!wm_clock(&now)) { wm_error(ctx, WM_CLOCK, GetLastError()); return FALSE; }
    if (now < ctx->result.start ||
        !wm_remaining(now, ctx->result.deadline, ctx->result.frequency, remaining)) {
        wm_error(ctx, WM_CLOCK, ERROR_ARITHMETIC_OVERFLOW); return FALSE;
    }
    if (!*remaining) { wm_error(ctx, WM_TIMEOUT, WAIT_TIMEOUT); return FALSE; }
    return TRUE;
}
static BOOL peak(WMContext *ctx) {
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION info;
    DWORD bytes = 0, active;
    uint32_t remaining;
    if (!wm_clock(&ctx->result.end)) { wm_error(ctx, WM_CLOCK, GetLastError()); return FALSE; }
    if (ctx->result.end < ctx->result.start) { wm_error(ctx, WM_CLOCK, ERROR_INVALID_DATA); return FALSE; }
    ctx->result.wall = ctx->result.end - ctx->result.start; ctx->result.end_known = TRUE;
    if (!budget(ctx, &remaining)) return FALSE;
    ZeroMemory(&info, sizeof(info));
    if (!QueryInformationJobObject(ctx->job, JobObjectExtendedLimitInformation, &info, (DWORD)sizeof(info), &bytes)) {
        wm_error(ctx, WM_JOB_QUERY, GetLastError()); return FALSE;
    }
    if (bytes != sizeof(info)) { wm_error(ctx, WM_JOB_QUERY, ERROR_INVALID_DATA); return FALSE; }
    ctx->result.peak = (uint64_t)info.PeakJobMemoryUsed; ctx->result.peak_known = TRUE;
    if (!wm_active(ctx, &active)) { wm_error(ctx, WM_JOB_QUERY, GetLastError()); return FALSE; }
    ctx->result.job_empty = active == 0;
    if (active) { wm_error(ctx, WM_JOB_QUERY, ERROR_INVALID_DATA); return FALSE; }
    return budget(ctx, &remaining);
}
BOOL wm_measure(WMContext *ctx) {
    for (;;) {
        HANDLE waits[3]; DWORD count = 0, active, waited;
        uint32_t remaining;
        if (!budget(ctx, &remaining) || !main_exit(ctx)) return FALSE;
        for (unsigned int i = 0; i < 2; ++i) {
            WMStream *s = &ctx->streams[i];
            if (s->pending && !wm_complete(ctx, s)) return FALSE;
            if (!wm_arm(ctx, s)) return FALSE;
        }
        if (ctx->result.child_exit_known && ctx->streams[0].eof && ctx->streams[1].eof) {
            if (!wm_active(ctx, &active)) { wm_error(ctx, WM_JOB_QUERY, GetLastError()); return FALSE; }
            ctx->result.job_empty = active == 0;
            if (!active) return peak(ctx);
        }
        if (!budget(ctx, &remaining)) return FALSE;
        if (!ctx->result.child_exit_known) waits[count++] = ctx->process.hProcess;
        for (unsigned int i = 0; i < 2; ++i)
            if (ctx->streams[i].pending) waits[count++] = ctx->streams[i].event;
        DWORD pause = remaining < 10u ? remaining : 10u;
        if (!count) { Sleep(pause); continue; }
        waited = WaitForMultipleObjects(count, waits, FALSE, pause);
        if (waited == WAIT_FAILED || (waited != WAIT_TIMEOUT && waited >= WAIT_OBJECT_0 + count)) {
            wm_error(ctx, WM_WAIT, waited == WAIT_FAILED ? GetLastError() : ERROR_INVALID_DATA); return FALSE;
        }
    }
}
