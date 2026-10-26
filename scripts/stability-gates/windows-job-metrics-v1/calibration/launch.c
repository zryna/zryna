#include "fixture.h"
BOOL fx_close_pi(PROCESS_INFORMATION *pi) {
    BOOL ok=fx_close(&pi->hThread);if (!fx_close(&pi->hProcess)) ok=FALSE;return ok;
}
BOOL fx_launch(const wchar_t *exe,const wchar_t *const *args,DWORD count,
               HANDLE job,enum FXStd mode,HANDLE out,HANDLE err,PROCESS_INFORMATION *pi) {
    static uint16_t command[WM_COMMAND_UNITS];
    const uint16_t *items[WM_MAX_ARGS];uint32_t used;
    HANDLE source[3]={NULL,NULL,NULL},allowed[3]={NULL,NULL,NULL};BOOL own[3]={FALSE,FALSE,FALSE};
    SIZE_T bytes=0;LPPROC_THREAD_ATTRIBUTE_LIST attributes=NULL;STARTUPINFOEXW start;
    BOOL initialized=FALSE,created=FALSE,ok=TRUE;DWORD saved=ERROR_INVALID_DATA;
    if (!count||count>WM_MAX_ARGS) return FALSE;
    ZeroMemory(pi,sizeof(*pi));
    for (DWORD i=0;i<count;++i) items[i]=(const uint16_t*)args[i];
    if (!wm_quote(items,count,command,WM_COMMAND_UNITS,&used)) return FALSE;
    for (DWORD i=0;i<3;++i) {
        if (mode==FX_INHERIT) source[i]=GetStdHandle(i==0?STD_INPUT_HANDLE:(i==1?STD_OUTPUT_HANDLE:STD_ERROR_HANDLE));
        else if (mode==FX_FILES&&i>0) source[i]=i==1?out:err;
        else {source[i]=CreateFileW(L"NUL",i==0?GENERIC_READ:GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,NULL,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,NULL);own[i]=TRUE;}
        if (!fx_valid(source[i])||!DuplicateHandle(GetCurrentProcess(),source[i],GetCurrentProcess(),&allowed[i],0,TRUE,DUPLICATE_SAME_ACCESS)) goto done;
    }
    if (InitializeProcThreadAttributeList(NULL,job?2u:1u,0,&bytes)||GetLastError()!=ERROR_INSUFFICIENT_BUFFER||!bytes||bytes>65536u) goto done;
    attributes=HeapAlloc(GetProcessHeap(),0,bytes);if (!attributes) goto done;
    if (!InitializeProcThreadAttributeList(attributes,job?2u:1u,0,&bytes)) goto done;
    initialized=TRUE;
    if (!UpdateProcThreadAttribute(attributes,0,PROC_THREAD_ATTRIBUTE_HANDLE_LIST,allowed,sizeof(allowed),NULL,NULL)) goto done;
    if (job&&!UpdateProcThreadAttribute(attributes,0,PROC_THREAD_ATTRIBUTE_JOB_LIST,&job,sizeof(job),NULL,NULL)) goto done;
    ZeroMemory(&start,sizeof(start));start.StartupInfo.cb=(DWORD)sizeof(start);start.StartupInfo.dwFlags=STARTF_USESTDHANDLES;
    start.StartupInfo.hStdInput=allowed[0];start.StartupInfo.hStdOutput=allowed[1];start.StartupInfo.hStdError=allowed[2];start.lpAttributeList=attributes;
    created=CreateProcessW(exe,(wchar_t*)command,NULL,NULL,TRUE,EXTENDED_STARTUPINFO_PRESENT|CREATE_UNICODE_ENVIRONMENT|CREATE_NO_WINDOW,NULL,NULL,&start.StartupInfo,pi);
 done:
    saved=created?ERROR_SUCCESS:GetLastError();
    for (DWORD i=0;i<3;++i) {
        if (!fx_close(&allowed[i])) {if(ok&&created)saved=GetLastError();ok=FALSE;}
        if (own[i]&&!fx_close(&source[i])) {if(ok&&created)saved=GetLastError();ok=FALSE;}
    }
    if (initialized) DeleteProcThreadAttributeList(attributes);
    if (attributes&&!HeapFree(GetProcessHeap(),0,attributes)) {if(ok&&created)saved=ERROR_INVALID_DATA;ok=FALSE;}
    if (!created||!ok) {SetLastError(saved?saved:ERROR_INVALID_DATA);return FALSE;}return TRUE;
}
