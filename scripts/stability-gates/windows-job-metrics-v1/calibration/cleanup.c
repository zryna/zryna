#include "fixture.h"
BOOL fx_active(HANDLE job,DWORD *active){
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION info;DWORD bytes=0;ZeroMemory(&info,sizeof(info));
    if(!QueryInformationJobObject(job,JobObjectBasicAccountingInformation,&info,(DWORD)sizeof(info),&bytes))return FALSE;
    if(bytes!=sizeof(info)){SetLastError(ERROR_INVALID_DATA);return FALSE;}*active=info.ActiveProcesses;return TRUE;
}
static void cleanup_error(FXOwner *o,DWORD error){if(!o->cleanup_error)o->cleanup_error=error?error:ERROR_INVALID_DATA;}
static void close_owned(FXOwner *o,HANDLE *handle){if(!fx_close(handle))cleanup_error(o,GetLastError());}
void fx_owner_cleanup(FXOwner *o){
    ULONGLONG began=GetTickCount64();BOOL need_kill=o->stage!=0;DWORD active=0;
    if(fx_valid(o->job)){
        if(!fx_active(o->job,&active)){cleanup_error(o,GetLastError());need_kill=TRUE;}
        if(active)need_kill=TRUE;
        if(need_kill){o->cleanup_forced=TRUE;if(!TerminateJobObject(o->job,FX_ABORT))cleanup_error(o,GetLastError());}
        for(;;){
            if(!fx_active(o->job,&active)){cleanup_error(o,GetLastError());break;}
            if(!active)break;
            if(GetTickCount64()-began>=FX_CLEANUP_MS){cleanup_error(o,WAIT_TIMEOUT);break;}Sleep(1);
        }
    }
    if(o->control.view)memcpy(&o->snapshot,o->control.view,sizeof(o->snapshot));
    /* Close kill-on-close ownership BEFORE artifact flushing, including failed-drain paths. */
    close_owned(o,&o->job);
    for(DWORD i=0;i<FX_SLOTS;++i)close_owned(o,&o->member[i]);
    if(!fx_close_pi(&o->helper))cleanup_error(o,GetLastError());
    if(!fx_control_close(&o->control))cleanup_error(o,GetLastError());
    if(fx_valid(o->helper_out)&&!FlushFileBuffers(o->helper_out))cleanup_error(o,GetLastError());
    if(fx_valid(o->helper_err)&&!FlushFileBuffers(o->helper_err))cleanup_error(o,GetLastError());
    close_owned(o,&o->helper_out);close_owned(o,&o->helper_err);
    if(GetTickCount64()-began>=FX_CLEANUP_MS)cleanup_error(o,WAIT_TIMEOUT);
    o->cleanup_confirmed=!o->cleanup_error&&!active;
}
