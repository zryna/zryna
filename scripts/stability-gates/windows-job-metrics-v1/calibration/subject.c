#include "fixture.h"
static BOOL bytes(HANDLE handle,const void *data,DWORD count) {
    DWORD at=0;
    while (at<count) {DWORD wrote=0;if(!WriteFile(handle,(const unsigned char*)data+at,count-at,&wrote,NULL)||!wrote||wrote>count-at)return FALSE;at+=wrote;}
    return TRUE;
}
static BOOL text(DWORD which,const char *value) {return bytes(GetStdHandle(which),value,(DWORD)strlen(value));}
static BOOL repeated(DWORD which,unsigned char value,DWORD total) {
    unsigned char block[4096];memset(block,value,sizeof(block));
    while(total){DWORD n=total>sizeof(block)?(DWORD)sizeof(block):total;if(!bytes(GetStdHandle(which),block,n))return FALSE;total-=n;}return TRUE;
}
static BOOL children_close(PROCESS_INFORMATION child[2],DWORD count,BOOL force) {
    HANDLE waits[2];DWORD n=0;BOOL ok=TRUE;
    for(DWORD i=0;i<count;++i)if(fx_valid(child[i].hProcess))waits[n++]=child[i].hProcess;
    if(force)for(DWORD i=0;i<n;++i)if(WaitForSingleObject(waits[i],0)==WAIT_TIMEOUT)
        if(!TerminateProcess(waits[i],FX_ABORT))ok=FALSE;
    if(n&&WaitForMultipleObjects(n,waits,TRUE,FX_OPERATION_MS)!=WAIT_OBJECT_0){
        ok=FALSE;
        for(DWORD i=0;i<n;++i)if(WaitForSingleObject(waits[i],0)==WAIT_TIMEOUT)(void)TerminateProcess(waits[i],FX_ABORT);
        (void)WaitForMultipleObjects(n,waits,TRUE,FX_CLEANUP_MS);
    }
    for(DWORD i=0;i<count;++i){DWORD code=0;
        if(fx_valid(child[i].hProcess)&&(!GetExitCodeProcess(child[i].hProcess,&code)||code==STILL_ACTIVE||(!force&&code)))ok=FALSE;
        if(!fx_close_pi(&child[i]))ok=FALSE;
    }return ok;
}
static BOOL start_child(FXControl *c,DWORD id,DWORD slot,PROCESS_INFORMATION *pi) {
    static wchar_t self[WM_COMMAND_UNITS];wchar_t number[2]={(wchar_t)('0'+slot),0};
    DWORD length=GetModuleFileNameW(NULL,self,WM_COMMAND_UNITS);
    if(!length||length>=WM_COMMAND_UNITS)return FALSE;
    const wchar_t *args[]={self,L"leaf",fx_name(id),number,c->token};
    return fx_launch(self,args,5,NULL,id==FX_EOF?FX_INHERIT:FX_NUL,NULL,NULL,pi);
}
DWORD fx_subject(int argc,wchar_t **argv) {
    FXControl c={0};PROCESS_INFORMATION child[2]={{0},{0}};DWORD count=0,result=93,id;
    if(argc<4||(id=fx_case(argv[2]))==0)return 91;
    if(id==FX_SUCCESS){
        static const wchar_t *const expected[]={L"",L"space argument",L"quote\"arg",L"trail\\",L"\x03a9\xd83d\xde00"};
        if(argc!=9)return 91;for(int i=0;i<5;++i)if(wcscmp(argv[4+i],expected[i]))return 91;
    }else if(argc!=4)return 91;
    if(!fx_control_open(&c,argv[3],id)||!fx_register(&c,0))goto done;
    if(id==FX_SUCCESS||id==FX_EXIT17){
        if(!fx_ready(&c,0)||!text(STD_OUTPUT_HANDLE,id==FX_SUCCESS?"CALIBRATION_OK\n":"CALIBRATION_EXIT17\n")||!text(STD_ERROR_HANDLE,"CALIBRATION_NOTE\n"))goto done;
        result=id==FX_SUCCESS?0:17;goto done;
    }
    if(id==FX_EXACT||id==FX_EXTRA){
        if(!fx_ready(&c,0)||!repeated(STD_OUTPUT_HANDLE,'O',8u*1024u*1024u)||!repeated(STD_ERROR_HANDLE,'E',8u*1024u*1024u))goto done;
        if(id==FX_EXTRA&&!repeated(STD_OUTPUT_HANDLE,'O',1))goto done;
        result=0;goto done;
    }
    for(DWORD slot=1;slot<fx_slots(id);++slot){
        BOOL launched=start_child(&c,id,slot,&child[count]);
        if(fx_valid(child[count].hProcess))++count;
        if(!launched)goto done;
    }
    for(DWORD slot=1;slot<fx_slots(id);++slot)
        if(WaitForSingleObject(c.event[FX_READY(slot)],FX_OPERATION_MS)!=WAIT_OBJECT_0)goto done;
    if(!fx_ready(&c,0))goto done;
    if(id==FX_EOF||id==FX_STRIPPED){
        if(!text(STD_OUTPUT_HANDLE,"MAIN_EXIT\n"))goto done;
        /* Supervisor already acknowledged the leaf's original handle before READY. */
        for(DWORD i=0;i<count;++i)if(!fx_close_pi(&child[i]))goto done;
        count=0;result=0;goto done;
    }
    if(id==FX_TIMEOUT){
        if(!text(STD_OUTPUT_HANDLE,"TIMEOUT_READY\n"))goto done;
        (void)WaitForSingleObject(c.event[FX_RELEASE],FX_OPERATION_MS);
        goto done; /* Any release/expiry is a fixture failure; expected helper kills first. */
    }
    if(WaitForSingleObject(c.event[FX_RELEASE],FX_OPERATION_MS)!=WAIT_OBJECT_0)goto done;
    if(!children_close(child,count,FALSE))goto done;
    count=0;if(!text(STD_OUTPUT_HANDLE,"MEMORY_OK\n"))goto done;result=0;
 done:
    if(count&&!children_close(child,count,TRUE))result=93;
    if(c.view){uint64_t stamp;if(!fx_qpc(&stamp))result=93;else c.view->slot[0].end_qpc=stamp;}
    if(!fx_control_close(&c))result=93;return result;
}
DWORD fx_leaf(int argc,wchar_t **argv) {
    FXControl c={0};DWORD id,slot,result=93,requested=0;unsigned char *memory=NULL;
    if(argc!=5||(id=fx_case(argv[2]))==0||wcslen(argv[3])!=1||argv[3][0]<'1'||argv[3][0]>'2')return 91;
    slot=(DWORD)(argv[3][0]-'0');
    if(slot>=fx_slots(id)||!fx_control_open(&c,argv[4],id)||!fx_register(&c,slot))goto done;
    if(id==FX_MEMORY){
        SYSTEM_INFO info;GetSystemInfo(&info);requested=slot==1?32u*1024u*1024u:64u*1024u*1024u;
        if(!info.dwPageSize||info.dwPageSize>requested)goto done;
        memory=VirtualAlloc(NULL,requested,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE);if(!memory)goto done;
        for(SIZE_T i=0;i<requested;i+=info.dwPageSize)((volatile unsigned char*)memory)[i]=0x5a;
        ((volatile unsigned char*)memory)[requested-1u]=0x5a;c.view->slot[slot].request_bytes=requested;
    }
    if(!fx_ready(&c,slot)||WaitForSingleObject(c.event[FX_RELEASE],FX_OPERATION_MS)!=WAIT_OBJECT_0)goto done;
    if(id==FX_EOF&&!text(STD_OUTPUT_HANDLE,"DESCENDANT_EXIT\n"))goto done;
    if(id==FX_TIMEOUT)goto done;result=0;
 done:
    if(memory&&!VirtualFree(memory,0,MEM_RELEASE))result=93;
    if(c.view){uint64_t stamp;if(!fx_qpc(&stamp))result=93;else c.view->slot[slot].end_qpc=stamp;}
    if(!fx_control_close(&c))result=93;return result;
}
