#include "fixture.h"
void fx_fail(FXOwner *o,DWORD stage,DWORD error){if(!o->stage){o->stage=stage;o->error=error;}}
BOOL fx_wait(FXOwner *o,HANDLE handle){
    ULONGLONG now=GetTickCount64();if(now>=o->deadline){fx_fail(o,20,WAIT_TIMEOUT);return FALSE;}
    DWORD wait=(DWORD)(o->deadline-now),result=WaitForSingleObject(handle,wait);
    if(result!=WAIT_OBJECT_0){fx_fail(o,20,result==WAIT_FAILED?GetLastError():WAIT_TIMEOUT);return FALSE;}return TRUE;
}
static BOOL event_or_helper(FXOwner *o,HANDLE event){
    HANDLE waits[2]={event,o->helper.hProcess};ULONGLONG now=GetTickCount64();
    if(now>=o->deadline){fx_fail(o,21,WAIT_TIMEOUT);return FALSE;}
    DWORD result=WaitForMultipleObjects(2,waits,FALSE,(DWORD)(o->deadline-now));
    if(result!=WAIT_OBJECT_0){fx_fail(o,21,result==WAIT_FAILED?GetLastError():ERROR_PROCESS_ABORTED);return FALSE;}return TRUE;
}
static BOOL bind_member(FXOwner *o,DWORD slot){
    FILETIME created,exit,kernel,user;FXSlot *s=&o->control.view->slot[slot];
    if(!event_or_helper(o,o->control.event[FX_PUBLISHED(slot)])||InterlockedCompareExchange(&s->published,0,0)!=1)return FALSE;
    o->member[slot]=OpenProcess(SYNCHRONIZE|PROCESS_QUERY_INFORMATION|PROCESS_VM_READ,FALSE,s->pid);
    if(!fx_valid(o->member[slot])||GetProcessId(o->member[slot])!=s->pid||
       !GetProcessTimes(o->member[slot],&created,&exit,&kernel,&user)||
       CompareFileTime(&created,&s->creation)!=0||WaitForSingleObject(o->member[slot],0)!=WAIT_TIMEOUT){fx_fail(o,22,GetLastError());return FALSE;}
    o->bound[slot]=TRUE;o->observed_mask|=1u<<slot;
    if(!SetEvent(o->control.event[FX_OBSERVED(slot)])){fx_fail(o,22,GetLastError());return FALSE;}return TRUE;
}
static BOOL peak_snapshot(FXOwner *o){
    for(DWORD slot=0;slot<o->slots;++slot){
        PROCESS_MEMORY_COUNTERS_EX info;ZeroMemory(&info,sizeof(info));info.cb=(DWORD)sizeof(info);
        if(WaitForSingleObject(o->member[slot],0)!=WAIT_TIMEOUT||
           !GetProcessMemoryInfo(o->member[slot],(PROCESS_MEMORY_COUNTERS*)&info,(DWORD)sizeof(info))){fx_fail(o,23,GetLastError());return FALSE;}
        o->process_peak[slot]=(uint64_t)info.PeakPagefileUsage;
    }o->peak_snapshot=TRUE;return TRUE;
}
BOOL fx_observe(FXOwner *o){
    for(DWORD slot=0;slot<o->slots;++slot)if(!bind_member(o,slot))return FALSE;
    for(DWORD slot=0;slot<o->slots;++slot){
        if(!event_or_helper(o,o->control.event[FX_READY(slot)])||
           InterlockedCompareExchange(&o->control.view->slot[slot].ready,0,0)!=1){fx_fail(o,24,ERROR_INVALID_DATA);return FALSE;}
        o->ready_mask|=1u<<slot;
    }
    if(o->case_id==FX_MEMORY&&!peak_snapshot(o))return FALSE;
    if(o->case_id==FX_EOF||o->case_id==FX_STRIPPED){
        if(!fx_wait(o,o->member[0]))return FALSE;o->main_before_release=TRUE;
        if(WaitForSingleObject(o->member[1],0)!=WAIT_TIMEOUT){fx_fail(o,25,ERROR_PROCESS_ABORTED);return FALSE;}
        DWORD status=WaitForSingleObject(o->helper.hProcess,FX_HELD_MS);
        if(status==WAIT_OBJECT_0)o->helper_while_held=TRUE;
        else if(status!=WAIT_TIMEOUT){fx_fail(o,25,GetLastError());return FALSE;}
    }
    if(o->case_id==FX_MEMORY||o->case_id==FX_EOF||o->case_id==FX_STRIPPED){
        if(!fx_qpc(&o->release_qpc)||!SetEvent(o->control.event[FX_RELEASE])){fx_fail(o,26,GetLastError());return FALSE;}
    }
    if(!fx_wait(o,o->helper.hProcess)||!GetExitCodeProcess(o->helper.hProcess,&o->helper_exit)){fx_fail(o,27,GetLastError());return FALSE;}
    o->helper_exit_known=TRUE;
    /* Freeze original process handles before waiting for outer accounting or cleanup. */
    for(DWORD slot=0;slot<o->slots;++slot){
        DWORD status=WaitForSingleObject(o->member[slot],0);o->gone[slot]=status==WAIT_OBJECT_0;
        if(o->gone[slot]){if(!GetExitCodeProcess(o->member[slot],&o->member_exit[slot])){fx_fail(o,28,GetLastError());return FALSE;}o->exit_known[slot]=TRUE;}
    }
    for(;;){
        if(GetTickCount64()>=o->deadline){fx_fail(o,29,WAIT_TIMEOUT);return FALSE;}
        if(!fx_active(o->job,&o->outer_before)){fx_fail(o,29,GetLastError());return FALSE;}
        o->outer_before_known=TRUE;
        if(GetTickCount64()>=o->deadline){fx_fail(o,29,WAIT_TIMEOUT);return FALSE;}
        if(!o->outer_before)return TRUE;
        Sleep(1);
    }
}
