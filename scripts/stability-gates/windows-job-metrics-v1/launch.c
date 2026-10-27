#include "win.h"
static void close_write(WMContext *ctx, HANDLE *handle) {
    if (wm_handle(*handle)) {
        if (CloseHandle(*handle)) *handle = NULL;
        else wm_error(ctx, WM_HANDLE_CLOSE, GetLastError());
    }
}
static BOOL clock_begin(WMContext *ctx) {
    LARGE_INTEGER frequency;
    if (!QueryPerformanceFrequency(&frequency) || frequency.QuadPart <= 0) {
        wm_error(ctx, WM_CLOCK, ERROR_INVALID_DATA); return FALSE;
    }
    ctx->result.frequency = (uint64_t)frequency.QuadPart; ctx->result.frequency_known = TRUE;
    if (!wm_clock(&ctx->result.start)) { wm_error(ctx, WM_CLOCK, GetLastError()); return FALSE; }
    ctx->result.start_known = TRUE;
    if (!wm_deadline(ctx->result.start, ctx->result.frequency, ctx->timeout_ms, &ctx->result.deadline)) {
        wm_error(ctx, WM_CLOCK, ERROR_ARITHMETIC_OVERFLOW); return FALSE;
    }
    return TRUE;
}
static BOOL job(WMContext *ctx) {
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION info;
    DWORD flags = 0;
    ctx->job = CreateJobObjectW(NULL, NULL);
    if (!wm_handle(ctx->job)) { wm_error(ctx, WM_JOB, GetLastError()); return FALSE; }
    if (!GetHandleInformation(ctx->job, &flags)) { wm_error(ctx, WM_JOB, GetLastError()); return FALSE; }
    if (flags & HANDLE_FLAG_INHERIT) { wm_error(ctx, WM_JOB, ERROR_INVALID_DATA); return FALSE; }
    ZeroMemory(&info, sizeof(info));
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!SetInformationJobObject(ctx->job, JobObjectExtendedLimitInformation, &info, (DWORD)sizeof(info))) {
        wm_error(ctx, WM_JOB, GetLastError()); return FALSE;
    }
    return TRUE;
}
BOOL wm_spawn(WMContext *ctx) {
    STARTUPINFOEXW startup;
    SIZE_T bytes = 0;
    HANDLE inherited[3] = {ctx->input, ctx->streams[0].write, ctx->streams[1].write};
    LPPROC_THREAD_ATTRIBUTE_LIST attributes = NULL;
    BOOL initialized = FALSE, created = FALSE;
    if (!job(ctx)) return FALSE;
    if (InitializeProcThreadAttributeList(NULL, 2, 0, &bytes) ||
        GetLastError() != ERROR_INSUFFICIENT_BUFFER || !bytes || bytes > 65536u) {
        wm_error(ctx, WM_ATTRIBUTES, ERROR_INVALID_DATA); return FALSE;
    }
    attributes = HeapAlloc(GetProcessHeap(), 0, bytes);
    if (!attributes) { wm_error(ctx, WM_ATTRIBUTES, ERROR_NOT_ENOUGH_MEMORY); return FALSE; }
    if (!InitializeProcThreadAttributeList(attributes, 2, 0, &bytes)) {
        wm_error(ctx, WM_ATTRIBUTES, GetLastError()); goto done;
    }
    initialized = TRUE;
    if (!UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                                   inherited, sizeof(inherited), NULL, NULL) ||
        !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_JOB_LIST,
                                   &ctx->job, sizeof(ctx->job), NULL, NULL)) {
        wm_error(ctx, WM_ATTRIBUTES, GetLastError()); goto done;
    }
    ZeroMemory(&startup, sizeof(startup));
    startup.StartupInfo.cb = (DWORD)sizeof(startup);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = ctx->input;
    startup.StartupInfo.hStdOutput = ctx->streams[0].write;
    startup.StartupInfo.hStdError = ctx->streams[1].write;
    startup.lpAttributeList = attributes;
    if (!clock_begin(ctx)) goto done;
    created = CreateProcessW(ctx->executable, (wchar_t *)ctx->command, NULL, NULL, TRUE,
        EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
        NULL, NULL, &startup.StartupInfo, &ctx->process);
    if (!created) wm_error(ctx, WM_SPAWN, GetLastError());
    ctx->result.spawned = created;
    /* Only these parent write handles can prevent EOF after child inheritance. */
    close_write(ctx, &ctx->streams[0].write); close_write(ctx, &ctx->streams[1].write);
    close_write(ctx, &ctx->input); close_write(ctx, &ctx->process.hThread);
 done:
    if (initialized) DeleteProcThreadAttributeList(attributes);
    if (!HeapFree(GetProcessHeap(), 0, attributes)) wm_error(ctx, WM_ATTRIBUTES, ERROR_INVALID_DATA);
    return created && !ctx->result.stage;
}
