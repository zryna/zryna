/* Private read-only observation, not host admission, quota discovery or a command launcher. */
#ifndef _M_X64
#error This observation recipe requires an x64 MSVC developer environment.
#endif
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tlhelp32.h>
#include <stdint.h>
#include <stdio.h>
#include <wchar.h>
#define QUERY_MS 10000ULL
#define SNAPSHOT_ROWS 8192U
#define OUTPUT_BYTES 2048U
/* Same 8192-record observation ceiling as the existing owned-process ancestry reader. */
typedef struct {
    WORD group;
    USHORT groups;
    DWORD active;
    DWORD_PTR process_mask, system_mask;
} affinity;
typedef struct {
    DWORD stage, error, cleanup_error, self_pid, parent_pid;
    uint64_t child_created, parent_created, parent_created_after, elapsed;
    BOOL live_before, live_after, matching, snapshot_closed, parent_closed;
    affinity before, after;
} observation;
static uint64_t filetime(FILETIME value) {
    return ((uint64_t)value.dwHighDateTime << 32) | value.dwLowDateTime;
}
static BOOL times(HANDLE process, uint64_t *created) {
    FILETIME birth, exit, kernel, user;
    if (!GetProcessTimes(process, &birth, &exit, &kernel, &user)) return FALSE;
    *created = filetime(birth);
    return *created != 0;
}
static DWORD failure(void) {
    const DWORD error = GetLastError();
    return error ? error : ERROR_GEN_FAILURE;
}
static BOOL fail(observation *o, DWORD stage, DWORD error) {
    if (!o->stage) { o->stage = stage; o->error = error ? error : ERROR_GEN_FAILURE; }
    return FALSE;
}
static BOOL deadline(observation *o, ULONGLONG started) {
    o->elapsed = GetTickCount64() - started;
    return o->elapsed < QUERY_MS || fail(o, 10, ERROR_TIMEOUT);
}
static BOOL pid_argument(int argc, wchar_t **argv, DWORD *pid) {
    uint64_t value = 0;
    if (argc != 3 || wcscmp(argv[1], L"--parent-pid") != 0) return FALSE;
    const size_t length = wcslen(argv[2]);
    if (!length || length > 10 || argv[2][0] == L'0') return FALSE;
    for (size_t i = 0; i < length; ++i) {
        if (argv[2][i] < L'0' || argv[2][i] > L'9') return FALSE;
        value = value * 10 + (uint64_t)(argv[2][i] - L'0');
    }
    if (!value || value > MAXDWORD) return FALSE;
    *pid = (DWORD)value;
    return TRUE;
}
static BOOL creating_parent(observation *o, ULONGLONG started) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return fail(o, 2, failure());
    PROCESSENTRY32W entry = {0};
    entry.dwSize = (DWORD)sizeof(entry);
    BOOL next = Process32FirstW(snapshot, &entry), found = FALSE;
    DWORD error = ERROR_NOT_FOUND;
    for (unsigned rows = 0; next && rows < SNAPSHOT_ROWS; ++rows) {
        if (!deadline(o, started)) break;
        if (entry.th32ProcessID == o->self_pid) {
            found = entry.th32ParentProcessID == o->parent_pid;
            error = ERROR_INVALID_DATA;
            break;
        }
        next = Process32NextW(snapshot, &entry);
        if (!next && GetLastError() != ERROR_NO_MORE_FILES) error = failure();
    }
    o->snapshot_closed = CloseHandle(snapshot);
    if (!o->snapshot_closed) return fail(o, 3, failure());
    return found || fail(o, 2, error);
}
static unsigned bits(DWORD_PTR value) {
    unsigned count = 0;
    while (value) { count += (unsigned)(value & 1U); value >>= 1; }
    return count;
}
static BOOL read_affinity(observation *o, HANDLE process, affinity *a) {
    a->groups = 1;
    if (!GetProcessGroupAffinity(process, &a->groups, &a->group)) {
        const DWORD error = failure();
        return fail(o, 5, a->groups > 1 ? ERROR_NOT_SUPPORTED : error);
    }
    if (a->groups != 1 || a->group == ALL_PROCESSOR_GROUPS) return fail(o, 5, ERROR_NOT_SUPPORTED);
    if (!GetProcessAffinityMask(process, &a->process_mask, &a->system_mask)) return fail(o, 5, failure());
    a->active = GetActiveProcessorCount(a->group);
    if (!a->active || a->active > 64 || !a->process_mask || !a->system_mask ||
        (a->process_mask & ~a->system_mask) || bits(a->system_mask) > a->active)
        return fail(o, 5, ERROR_INVALID_DATA);
    USHORT count = 1;
    WORD group = 0;
    if (!GetProcessGroupAffinity(process, &count, &group)) return fail(o, 5, failure());
    return (count == 1 && group == a->group) || fail(o, 5, ERROR_INVALID_DATA);
}
static BOOL observe(observation *o, ULONGLONG started) {
    if (!times(GetCurrentProcess(), &o->child_created)) return fail(o, 1, failure());
    if (!creating_parent(o, started) || !deadline(o, started)) return FALSE;
    HANDLE parent = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, FALSE, o->parent_pid);
    if (!parent) return fail(o, 4, failure());
    BOOL ok = GetProcessId(parent) == o->parent_pid && times(parent, &o->parent_created) &&
        o->parent_created < o->child_created;
    if (!ok) fail(o, 4, failure());
    if (ok) {
        o->live_before = WaitForSingleObject(parent, 0) == WAIT_TIMEOUT;
        ok = o->live_before || fail(o, 4, ERROR_PROCESS_ABORTED);
    }
    if (ok) ok = read_affinity(o, parent, &o->before) && deadline(o, started);
    if (ok) ok = read_affinity(o, parent, &o->after) && deadline(o, started);
    if (ok) {
        ok = times(parent, &o->parent_created_after) && o->parent_created_after == o->parent_created;
        if (!ok) fail(o, 6, ERROR_PROCESS_ABORTED);
    }
    if (ok) {
        o->matching = o->before.group == o->after.group && o->before.groups == o->after.groups &&
            o->before.active == o->after.active && o->before.process_mask == o->after.process_mask &&
            o->before.system_mask == o->after.system_mask;
        ok = o->matching || fail(o, 6, ERROR_INVALID_DATA);
    }
    if (ok) {
        o->live_after = WaitForSingleObject(parent, 0) == WAIT_TIMEOUT;
        ok = o->live_after || fail(o, 6, ERROR_PROCESS_ABORTED);
    }
    o->parent_closed = CloseHandle(parent);
    if (!o->parent_closed) {
        o->cleanup_error = failure();
        ok = fail(o, 7, o->cleanup_error);
    }
    return deadline(o, started) && ok;
}
static BOOL emit(const observation *o) {
    char output[OUTPUT_BYTES];
    const int size = _snprintf_s(output, sizeof(output), _TRUNCATE,
        "{\"format\":\"zryna.windows-parent-affinity.v1\",\"status\":\"%s\","
        "\"stage\":%lu,\"error\":%lu,\"cleanupError\":%lu,\"childPid\":%lu,\"parentPid\":%lu,"
        "\"childCreated\":\"%llu\",\"parentCreated\":\"%llu\",\"parentCreatedAfter\":\"%llu\","
        "\"parentLiveBefore\":%s,\"parentLiveAfter\":%s,\"snapshotsMatch\":%s,"
        "\"snapshotClosed\":%s,\"parentHandleClosed\":%s,\"groupCount\":%u,\"groupId\":%u,"
        "\"activeProcessorCount\":%lu,\"processMask\":\"%llu\",\"systemMask\":\"%llu\","
        "\"maskBitCount\":%u,\"afterGroupCount\":%u,\"afterGroupId\":%u,\"afterActiveProcessorCount\":%lu,"
        "\"afterProcessMask\":\"%llu\",\"afterSystemMask\":\"%llu\",\"elapsedMs\":%llu}\n",
        o->stage ? "unavailable" : "observed", o->stage, o->error, o->cleanup_error, o->self_pid, o->parent_pid,
        (unsigned long long)o->child_created, (unsigned long long)o->parent_created,
        (unsigned long long)o->parent_created_after, o->live_before ? "true" : "false",
        o->live_after ? "true" : "false", o->matching ? "true" : "false",
        o->snapshot_closed ? "true" : "false", o->parent_closed ? "true" : "false",
        (unsigned)o->before.groups, (unsigned)o->before.group, o->before.active,
        (unsigned long long)o->before.process_mask, (unsigned long long)o->before.system_mask,
        bits(o->before.process_mask), (unsigned)o->after.groups, (unsigned)o->after.group, o->after.active,
        (unsigned long long)o->after.process_mask, (unsigned long long)o->after.system_mask,
        (unsigned long long)o->elapsed);
    if (size <= 0 || (unsigned)size >= OUTPUT_BYTES) return FALSE;
    HANDLE out = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD written = 0;
    return out && out != INVALID_HANDLE_VALUE && WriteFile(out, output, (DWORD)size, &written, NULL) && written == (DWORD)size;
}
int wmain(int argc, wchar_t **argv) {
    observation o = {0};
    const ULONGLONG started = GetTickCount64();
    o.self_pid = GetCurrentProcessId();
    if (!pid_argument(argc, argv, &o.parent_pid)) fail(&o, 1, ERROR_INVALID_PARAMETER);
    else (void)observe(&o, started);
    if (!emit(&o)) return 2;
    return o.stage ? 1 : 0;
}
