#include "win.h"
#ifndef _WIN64
#error This evidence helper is scoped to Windows x64 only.
#endif
/* Static lifetime survives incomplete kernel cancellation until final ExitProcess. */
static WMContext context;
int wmain(int argc, wchar_t **argv) {
    BOOL recorded;
    if (wm_options(&context, argc, argv) && wm_outputs(&context) &&
        wm_pipes(&context) && wm_spawn(&context)) (void)wm_measure(&context);
    wm_cleanup(&context);
    recorded = wm_metadata(&context);
    if (!recorded || !wm_sample_valid(&context)) {
        (void)fputs("m418 Windows Job metrics failed; retain owned output and tool exit status\n", stderr);
        ExitProcess(1);
    }
    ExitProcess(0);
}
