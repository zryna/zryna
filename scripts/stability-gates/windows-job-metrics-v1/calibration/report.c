#include "fixture.h"
#include <stdarg.h>
static BOOL append(char *out,size_t *at,const char *format,...){
    va_list args;va_start(args,format);int size=vsnprintf(out+*at,8192u-*at,format,args);va_end(args);
    if(size<0||(size_t)size>=8192u-*at)return FALSE;*at+=(size_t)size;return TRUE;
}
static const char *yes(BOOL value){return value?"true":"false";}
static const char *number(char out[32],BOOL known,uint64_t value){
    if(!known)return "null";(void)snprintf(out,32,"\"%" PRIu64 "\"",value);return out;
}
BOOL fx_report(FXOwner *o){
    static char out[8192];static wchar_t path[WM_COMMAND_UNITS];size_t at=0;char value[32];
    if(!o->directory_owned||!o->directory||_snwprintf_s(path,WM_COMMAND_UNITS,_TRUNCATE,L"%ls\\owner.json",o->directory)<0)return FALSE;
    if(!append(out,&at,"{\"format\":\"zryna.windows-job-calibration-owner.v1\",\"caseId\":%lu,\"slots\":%lu,\"errorStage\":%lu,\"errorWin32\":%lu,\"cleanupErrorWin32\":%lu,\"observedMask\":%lu,\"readyMask\":%lu,",o->case_id,o->slots,o->stage,o->error,o->cleanup_error,o->observed_mask,o->ready_mask))return FALSE;
    if(!append(out,&at,"\"helperExitKnown\":%s,\"helperExit\":%s,",yes(o->helper_exit_known),number(value,o->helper_exit_known,o->helper_exit)))return FALSE;
    if(!append(out,&at,"\"mainExitedBeforeRelease\":%s,\"helperExitedWhileHeld\":%s,\"peakSnapshot\":%s,\"outerBeforeKnown\":%s,\"outerBeforeActive\":%lu,\"cleanupForced\":%s,\"cleanupConfirmed\":%s,",yes(o->main_before_release),yes(o->helper_while_held),yes(o->peak_snapshot),yes(o->outer_before_known),o->outer_before,yes(o->cleanup_forced),yes(o->cleanup_confirmed)))return FALSE;
    if(!append(out,&at,"\"qpcFrequency\":%s,",number(value,o->frequency!=0,o->frequency)))return FALSE;
    if(!append(out,&at,"\"releaseQpc\":%s",number(value,o->release_qpc!=0,o->release_qpc)))return FALSE;
    for(DWORD i=0;i<FX_SLOTS;++i){FXSlot *s=&o->snapshot.slot[i];
        if(!append(out,&at,",\"pid%lu\":%lu,\"creationLow%lu\":%lu,\"creationHigh%lu\":%lu,\"bound%lu\":%s,\"goneBeforeCleanup%lu\":%s,\"exitKnown%lu\":%s,",i,s->pid,i,s->creation.dwLowDateTime,i,s->creation.dwHighDateTime,i,yes(o->bound[i]),i,yes(o->gone[i]),i,yes(o->exit_known[i])))return FALSE;
        if(!append(out,&at,"\"exit%lu\":%s,",i,number(value,o->exit_known[i],o->member_exit[i])))return FALSE;
        if(!append(out,&at,"\"readyQpc%lu\":%s,",i,number(value,s->ready_qpc!=0,s->ready_qpc)))return FALSE;
        if(!append(out,&at,"\"endQpc%lu\":%s,\"requestBytes%lu\":%lu,",i,number(value,s->end_qpc!=0,s->end_qpc),i,s->request_bytes))return FALSE;
        if(!append(out,&at,"\"peakCommitBytes%lu\":%s",i,number(value,o->peak_snapshot,o->process_peak[i])))return FALSE;
    }
    if(!append(out,&at,"}\n"))return FALSE;
    HANDLE file=CreateFileW(path,GENERIC_WRITE,FILE_SHARE_READ,NULL,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,NULL);DWORD wrote=0;
    if(!fx_valid(file))return FALSE;
    BOOL ok=WriteFile(file,out,(DWORD)at,&wrote,NULL)&&wrote==(DWORD)at&&FlushFileBuffers(file);
    if(!CloseHandle(file))ok=FALSE;return ok;
}
