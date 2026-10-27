#include "fixture.h"
BOOL fx_valid(HANDLE h) { return h && h != INVALID_HANDLE_VALUE; }
BOOL fx_close(HANDLE *h) { if (!fx_valid(*h)) return TRUE; if (!CloseHandle(*h)) return FALSE; *h=NULL; return TRUE; }
BOOL fx_qpc(uint64_t *value) {
    LARGE_INTEGER n; if (!QueryPerformanceCounter(&n) || n.QuadPart < 0) return FALSE;
    *value=(uint64_t)n.QuadPart; return TRUE;
}
DWORD fx_case(const wchar_t *name) {
    for (DWORD i=FX_SUCCESS;i<=FX_MEMORY;++i) if (!wcscmp(name,fx_name(i))) return i;
    return 0;
}
const wchar_t *fx_name(DWORD id) {
    static const wchar_t *const names[]={L"invalid",L"success",L"exit17",L"timeout",L"exact-limit",L"first-extra",L"held-eof",L"stripped-stdio",L"memory-overlap"};
    return id<=FX_MEMORY?names[id]:names[0];
}
DWORD fx_slots(DWORD id) { return id==FX_MEMORY?3u:(id==FX_TIMEOUT||id==FX_EOF||id==FX_STRIPPED?2u:1u); }
static BOOL object_name(wchar_t name[160], const wchar_t *token, int event) {
    return _snwprintf_s(name,160,_TRUNCATE,L"%ls-%d",token,event)>=0;
}
BOOL fx_control_create(FXControl *c, DWORD id, DWORD slots) {
    wchar_t name[160]; uint64_t stamp; DWORD last;
    if (!fx_qpc(&stamp) || _snwprintf_s(c->token,128,_TRUNCATE,L"Local\\zryna-m418-cal-%lu-%llu",GetCurrentProcessId(),(unsigned long long)stamp)<0) return FALSE;
    if (!object_name(name,c->token,-1)) return FALSE;
    SetLastError(ERROR_SUCCESS);
    c->mapping=CreateFileMappingW(INVALID_HANDLE_VALUE,NULL,PAGE_READWRITE,0,(DWORD)sizeof(FXShared),name);
    last=GetLastError(); if (!fx_valid(c->mapping)||last==ERROR_ALREADY_EXISTS) return FALSE;
    c->view=MapViewOfFile(c->mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,sizeof(FXShared));
    if (!c->view) return FALSE;
    ZeroMemory(c->view,sizeof(FXShared));
    c->view->magic=FX_MAGIC;c->view->version=1;c->view->case_id=id;c->view->slots=slots;
    for (unsigned int i=0;i<FX_EVENTS;++i) {
        if (!object_name(name,c->token,(int)i)) return FALSE;
        SetLastError(ERROR_SUCCESS);c->event[i]=CreateEventW(NULL,TRUE,FALSE,name);last=GetLastError();
        if (!fx_valid(c->event[i])||last==ERROR_ALREADY_EXISTS) return FALSE;
    }
    return TRUE;
}
BOOL fx_control_open(FXControl *c, const wchar_t *token, DWORD id) {
    wchar_t name[160]; size_t n=wcslen(token);
    if (n>=128||wcsncmp(token,L"Local\\zryna-m418-cal-",21)) return FALSE;
    for (size_t i=21;i<n;++i) if ((token[i]<'0'||token[i]>'9')&&token[i]!='-') return FALSE;
    if (wcscpy_s(c->token,128,token)||!object_name(name,token,-1)) return FALSE;
    c->mapping=OpenFileMappingW(FILE_MAP_READ|FILE_MAP_WRITE,FALSE,name);
    if (!fx_valid(c->mapping)) return FALSE;
    c->view=MapViewOfFile(c->mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,sizeof(FXShared));
    if (!c->view||c->view->magic!=FX_MAGIC||c->view->version!=1||c->view->case_id!=id||c->view->slots!=fx_slots(id)) return FALSE;
    for (unsigned int i=0;i<FX_EVENTS;++i) {
        if (!object_name(name,token,(int)i)) return FALSE;
        c->event[i]=OpenEventW(SYNCHRONIZE|EVENT_MODIFY_STATE,FALSE,name);
        if (!fx_valid(c->event[i])) return FALSE;
    }
    return TRUE;
}
BOOL fx_control_close(FXControl *c) {
    BOOL ok=TRUE;
    for (unsigned int i=0;i<FX_EVENTS;++i) if (!fx_close(&c->event[i])) ok=FALSE;
    if (c->view) { if (!UnmapViewOfFile(c->view)) ok=FALSE; else c->view=NULL; }
    if (!fx_close(&c->mapping)) ok=FALSE;return ok;
}
BOOL fx_register(FXControl *c, DWORD slot) {
    FILETIME creation,exit,kernel,user;
    if (slot>=c->view->slots||!GetProcessTimes(GetCurrentProcess(),&creation,&exit,&kernel,&user)) return FALSE;
    c->view->slot[slot].pid=GetCurrentProcessId();c->view->slot[slot].creation=creation;
    InterlockedExchange(&c->view->slot[slot].published,1);
    return SetEvent(c->event[FX_PUBLISHED(slot)])&&WaitForSingleObject(c->event[FX_OBSERVED(slot)],FX_OPERATION_MS)==WAIT_OBJECT_0;
}
BOOL fx_ready(FXControl *c, DWORD slot) {
    uint64_t stamp;if(!fx_qpc(&stamp))return FALSE;c->view->slot[slot].ready_qpc=stamp;
    InterlockedExchange(&c->view->slot[slot].ready,1);return SetEvent(c->event[FX_READY(slot)]);
}
