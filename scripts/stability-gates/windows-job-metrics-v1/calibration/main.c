#include "fixture.h"
#ifndef _WIN64
#error Windows x64 calibration only.
#endif
int wmain(int argc,wchar_t **argv){
    DWORD result=91;
    if(argc>=2){
        if(!wcscmp(argv[1],L"supervise"))result=fx_supervise(argc,argv);
        else if(!wcscmp(argv[1],L"subject"))result=fx_subject(argc,argv);
        else if(!wcscmp(argv[1],L"leaf"))result=fx_leaf(argc,argv);
    }
    ExitProcess(result);
}
