#include "fixture.h"
static BOOL path(wchar_t out[WM_COMMAND_UNITS],const wchar_t *directory,const wchar_t *name){return _snwprintf_s(out,WM_COMMAND_UNITS,_TRUNCATE,L"%ls\\%ls",directory,name)>=0;}
static HANDLE output(const wchar_t *directory,const wchar_t *name){
    static wchar_t name_path[WM_COMMAND_UNITS];if(!path(name_path,directory,name))return INVALID_HANDLE_VALUE;
    return CreateFileW(name_path,GENERIC_WRITE,FILE_SHARE_READ,NULL,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,NULL);
}
static BOOL setup(FXOwner *o){
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits;LARGE_INTEGER frequency;DWORD length;
    if(!CreateDirectoryW(o->directory,NULL)){fx_fail(o,1,GetLastError());return FALSE;}
    o->directory_owned=TRUE;
    DWORD attrs=GetFileAttributesW(o->directory);
    if(attrs==INVALID_FILE_ATTRIBUTES||(attrs&FILE_ATTRIBUTE_REPARSE_POINT)){fx_fail(o,1,ERROR_ACCESS_DENIED);return FALSE;}
    length=GetModuleFileNameW(NULL,o->self,WM_COMMAND_UNITS);
    if(!length||length>=WM_COMMAND_UNITS||!path(o->sample,o->directory,L"sample")||
       !QueryPerformanceFrequency(&frequency)||frequency.QuadPart<=0){fx_fail(o,2,GetLastError());return FALSE;}
    o->frequency=(uint64_t)frequency.QuadPart;
    if(!fx_control_create(&o->control,o->case_id,o->slots)){fx_fail(o,3,GetLastError());return FALSE;}
    o->job=CreateJobObjectW(NULL,NULL);if(!fx_valid(o->job)){fx_fail(o,4,GetLastError());return FALSE;}
    ZeroMemory(&limits,sizeof(limits));limits.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if(!SetInformationJobObject(o->job,JobObjectExtendedLimitInformation,&limits,(DWORD)sizeof(limits))){fx_fail(o,4,GetLastError());return FALSE;}
    DWORD flags=0;if(!GetHandleInformation(o->job,&flags)||(flags&HANDLE_FLAG_INHERIT)){fx_fail(o,4,ERROR_INVALID_DATA);return FALSE;}
    o->helper_out=output(o->directory,L"helper.stdout.raw");o->helper_err=output(o->directory,L"helper.stderr.raw");
    if(!fx_valid(o->helper_out)||!fx_valid(o->helper_err)){fx_fail(o,5,GetLastError());return FALSE;}return TRUE;
}
static BOOL start(FXOwner *o){
    wchar_t milliseconds[16];if(_snwprintf_s(milliseconds,16,_TRUNCATE,L"%lu",o->helper_ms)<0)return FALSE;
    const wchar_t *args[16]={o->helper_exe,L"--timeout-ms",milliseconds,L"--output",o->sample,L"--",o->self,L"subject",fx_name(o->case_id),o->control.token};
    DWORD count=10;
    if(o->case_id==FX_SUCCESS){args[count++]=L"";args[count++]=L"space argument";args[count++]=L"quote\"arg";args[count++]=L"trail\\";args[count++]=L"\x03a9\xd83d\xde00";}
    ULONGLONG now=GetTickCount64(),duration=(ULONGLONG)o->helper_ms+FX_CLEANUP_MS;
    if(now>UINT64_MAX-duration){fx_fail(o,6,ERROR_ARITHMETIC_OVERFLOW);return FALSE;}o->deadline=now+duration;
    if(!fx_launch(o->helper_exe,args,count,o->job,FX_FILES,o->helper_out,o->helper_err,&o->helper)){fx_fail(o,6,GetLastError());return FALSE;}
    if(!fx_close(&o->helper.hThread)){fx_fail(o,6,GetLastError());return FALSE;}
    return TRUE;
}
DWORD fx_supervise(int argc,wchar_t **argv){
    static FXOwner owner;
    if(argc!=5||!wm_local_path((const uint16_t*)argv[2])||!wm_local_path((const uint16_t*)argv[3])||(owner.case_id=fx_case(argv[4]))==0)return 91;
    owner.helper_exe=argv[2];owner.directory=argv[3];owner.slots=fx_slots(owner.case_id);owner.helper_ms=owner.case_id==FX_TIMEOUT?5000u:10000u;
    if(setup(&owner)&&start(&owner))if(!fx_observe(&owner)&&!owner.stage)fx_fail(&owner,30,ERROR_INVALID_DATA);
    fx_owner_cleanup(&owner);
    if(!fx_report(&owner)||owner.stage||owner.cleanup_error){(void)fputs("m418 calibration owner failed; retain output\n",stderr);return 1;}return 0;
}
