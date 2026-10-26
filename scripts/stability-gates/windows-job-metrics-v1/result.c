#include "win.h"
static const char *boolean(BOOL value) { return value ? "true" : "false"; }
static void number(char out[32], BOOL known, uint64_t value) {
    if (known) (void)snprintf(out, 32, "\"%" PRIu64 "\"", value);
    else { out[0] = 'n'; out[1] = 'u'; out[2] = 'l'; out[3] = 'l'; out[4] = 0; }
}
BOOL wm_sample_valid(WMContext *ctx) {
    WMResult *r = &ctx->result;
    return !r->stage && !r->cleanup_stage && r->cleanup_confirmed && !r->forced &&
        r->spawned && r->child_exit_known && !r->child_exit && r->job_empty &&
        ctx->streams[0].eof && ctx->streams[1].eof && r->peak_known && r->end_known;
}
BOOL wm_metadata(WMContext *ctx) {
    char record[4096], exit_code[32], peak[32], start[32], end[32], frequency[32], ticks[32];
    WMResult *r = &ctx->result; DWORD written = 0;
    if (!wm_handle(ctx->metadata)) return FALSE;
    number(exit_code, r->child_exit_known, r->child_exit); number(peak, r->peak_known, r->peak);
    number(start, r->start_known, r->start); number(end, r->end_known, r->end);
    number(frequency, r->frequency_known, r->frequency); number(ticks, r->end_known, r->wall);
    int size = snprintf(record, sizeof(record),
        "{\"format\":\"zryna.stability.windows-job-metrics.v1\","
        "\"metric\":\"job-peak-committed-bytes.v1\",\"measurementStatus\":\"%s\","
        "\"timeoutMs\":%lu,\"cleanupReserveMs\":%u,\"combinedStreamLimitBytes\":%u,"
        "\"childArgc\":%lu,\"commandUtf16Units\":%lu,\"childCreated\":%s,\"childPid\":%lu,"
        "\"mainExitKnown\":%s,\"mainExitCode\":%s,\"stdoutBytes\":%lu,\"stderrBytes\":%lu,"
        "\"stdoutEof\":%s,\"stderrEof\":%s,\"jobEmptyConfirmed\":%s,"
        "\"forcedTermination\":%s,\"cleanupConfirmed\":%s,\"peakQuerySucceeded\":%s,"
        "\"peakJobMemoryUsedBytes\":%s,\"qpcFrequency\":%s,\"qpcStart\":%s,\"qpcEnd\":%s,"
        "\"wallTicks\":%s,\"primaryStage\":%lu,\"primaryWin32Error\":%lu,"
        "\"cleanupStage\":%lu,\"cleanupWin32Error\":%lu}\n",
        wm_sample_valid(ctx) ? "complete" : "invalid", ctx->timeout_ms, WM_CLEANUP_MS, WM_LOG_BYTES,
        ctx->argc, ctx->command_units, boolean(r->spawned), ctx->process.dwProcessId,
        boolean(r->child_exit_known), exit_code, ctx->streams[0].bytes, ctx->streams[1].bytes,
        boolean(ctx->streams[0].eof), boolean(ctx->streams[1].eof), boolean(r->job_empty),
        boolean(r->forced), boolean(r->cleanup_confirmed), boolean(r->peak_known), peak,
        frequency, start, end, ticks, r->stage, r->error, r->cleanup_stage, r->cleanup_error);
    if (size < 0 || size >= (int)sizeof(record)) {
        wm_error(ctx, WM_METADATA, ERROR_BUFFER_OVERFLOW); return FALSE;
    }
    if (!WriteFile(ctx->metadata, record, (DWORD)size, &written, NULL)) {
        wm_error(ctx, WM_METADATA, GetLastError()); return FALSE;
    }
    if (written != (DWORD)size) { wm_error(ctx, WM_METADATA, ERROR_WRITE_FAULT); return FALSE; }
    if (!FlushFileBuffers(ctx->metadata)) { wm_error(ctx, WM_METADATA, GetLastError()); return FALSE; }
    if (!CloseHandle(ctx->metadata)) { wm_error(ctx, WM_METADATA, GetLastError()); return FALSE; }
    ctx->metadata = NULL; return TRUE;
}
