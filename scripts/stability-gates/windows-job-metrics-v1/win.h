#ifndef WM_WIN_H
#define WM_WIN_H
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <inttypes.h>
#include <stdio.h>
#include <wchar.h>
#include "portable.h"
_Static_assert(sizeof(wchar_t) == sizeof(uint16_t), "Win32 UTF16 required");
enum WMStage {
    WM_OK = 0, WM_OPTIONS, WM_OUTPUT, WM_PIPE, WM_JOB, WM_ATTRIBUTES,
    WM_CLOCK, WM_SPAWN, WM_READ, WM_WRITE, WM_OVERFLOW, WM_CHILD_EXIT,
    WM_TIMEOUT, WM_JOB_QUERY, WM_WAIT, WM_CANCEL, WM_TERMINATE, WM_HANDLE_CLOSE,
    WM_METADATA
};
typedef struct WMStream {
    HANDLE read, write, event, file;
    OVERLAPPED io;
    BOOL pending, eof;
    DWORD bytes;
    unsigned char buffer[4096];
} WMStream;
typedef struct WMResult {
    DWORD stage, error, cleanup_stage, cleanup_error, child_exit;
    BOOL child_exit_known, spawned, job_empty, forced, cleanup_confirmed;
    BOOL frequency_known, start_known, end_known, peak_known;
    uint64_t frequency, start, end, wall, peak, deadline;
} WMResult;
typedef struct WMContext {
    DWORD timeout_ms, argc, command_units;
    const wchar_t *directory, *executable;
    uint16_t command[WM_COMMAND_UNITS];
    HANDLE metadata, job, input;
    PROCESS_INFORMATION process;
    WMStream streams[2];
    WMResult result;
} WMContext;
BOOL wm_handle(HANDLE handle);
void wm_error(WMContext *ctx, DWORD stage, DWORD error);
void wm_cleanup_error(WMContext *ctx, DWORD stage, DWORD error);
BOOL wm_options(WMContext *ctx, int argc, wchar_t **argv);
BOOL wm_outputs(WMContext *ctx);
BOOL wm_pipes(WMContext *ctx);
BOOL wm_spawn(WMContext *ctx);
BOOL wm_clock(uint64_t *value);
BOOL wm_active(WMContext *ctx, DWORD *active);
BOOL wm_measure(WMContext *ctx);
BOOL wm_arm(WMContext *ctx, WMStream *stream);
BOOL wm_complete(WMContext *ctx, WMStream *stream);
void wm_cleanup(WMContext *ctx);
BOOL wm_sample_valid(WMContext *ctx);
BOOL wm_metadata(WMContext *ctx);
#endif
