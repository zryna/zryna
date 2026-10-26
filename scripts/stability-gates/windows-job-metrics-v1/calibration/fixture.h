#ifndef FX_FIXTURE_H
#define FX_FIXTURE_H
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif
#define WIN32_LEAN_AND_MEAN
#define PSAPI_VERSION 2
#include <windows.h>
#include <psapi.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>
#include "../portable.h"
#define FX_SLOTS 3u
#define FX_EVENTS 10u
#define FX_OPERATION_MS 10000u
#define FX_CLEANUP_MS 10000u
#define FX_HELD_MS 250u
#define FX_ABORT 0xe4180001u
#define FX_MAGIC 0x4d343138u
_Static_assert(sizeof(wchar_t) == 2, "UTF16 Windows x64 only");
enum FXCase { FX_SUCCESS=1, FX_EXIT17, FX_TIMEOUT, FX_EXACT, FX_EXTRA, FX_EOF, FX_STRIPPED, FX_MEMORY };
enum FXStd { FX_INHERIT, FX_NUL, FX_FILES };
#define FX_PUBLISHED(i) (i)
#define FX_OBSERVED(i) (3u+(i))
#define FX_READY(i) (6u+(i))
#define FX_RELEASE 9u
typedef struct FXSlot {
    DWORD pid; FILETIME creation; volatile LONG published, ready;
    DWORD request_bytes; uint64_t ready_qpc, end_qpc;
} FXSlot;
typedef struct FXShared { DWORD magic, version, case_id, slots; FXSlot slot[FX_SLOTS]; } FXShared;
_Static_assert(sizeof(FXShared) < 4096, "fixed control mapping");
typedef struct FXControl { HANDLE mapping, event[FX_EVENTS]; FXShared *view; wchar_t token[128]; } FXControl;
typedef struct FXOwner {
    FXControl control; FXShared snapshot;
    HANDLE job, member[FX_SLOTS], helper_out, helper_err;
    PROCESS_INFORMATION helper;
    const wchar_t *helper_exe, *directory;
    wchar_t self[WM_COMMAND_UNITS], sample[WM_COMMAND_UNITS];
    DWORD case_id, slots, helper_ms, observed_mask, ready_mask;
    DWORD stage, error, cleanup_error, helper_exit, member_exit[FX_SLOTS];
    BOOL helper_exit_known, bound[FX_SLOTS], gone[FX_SLOTS], exit_known[FX_SLOTS];
    BOOL main_before_release, helper_while_held, peak_snapshot, cleanup_forced, cleanup_confirmed;
    BOOL directory_owned, outer_before_known; DWORD outer_before;
    uint64_t frequency, release_qpc, process_peak[FX_SLOTS];
    ULONGLONG deadline;
} FXOwner;
BOOL fx_valid(HANDLE handle);
BOOL fx_close(HANDLE *handle);
BOOL fx_qpc(uint64_t *value);
DWORD fx_case(const wchar_t *name);
const wchar_t *fx_name(DWORD id);
DWORD fx_slots(DWORD id);
BOOL fx_control_create(FXControl *c, DWORD id, DWORD slots);
BOOL fx_control_open(FXControl *c, const wchar_t *token, DWORD id);
BOOL fx_control_close(FXControl *c);
BOOL fx_register(FXControl *c, DWORD slot);
BOOL fx_ready(FXControl *c, DWORD slot);
BOOL fx_launch(const wchar_t *exe, const wchar_t *const *args, DWORD count,
               HANDLE job, enum FXStd mode, HANDLE out, HANDLE err, PROCESS_INFORMATION *pi);
BOOL fx_close_pi(PROCESS_INFORMATION *pi);
DWORD fx_subject(int argc, wchar_t **argv);
DWORD fx_leaf(int argc, wchar_t **argv);
BOOL fx_observe(FXOwner *o);
void fx_fail(FXOwner *o, DWORD stage, DWORD error);
BOOL fx_wait(FXOwner *o, HANDLE handle);
BOOL fx_active(HANDLE job, DWORD *active);
void fx_owner_cleanup(FXOwner *o);
BOOL fx_report(FXOwner *o);
DWORD fx_supervise(int argc, wchar_t **argv);
#endif
