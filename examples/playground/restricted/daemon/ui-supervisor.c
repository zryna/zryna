/* Private trusted UI test role; production supervisor.c and its four modes stay separate. */
#include "supervisor_sys.h"
#include "ui-supervisor-limits.h"

static char *const ui_environment[]={"PATH=/materials/runtime/node/bin","LC_ALL=C.UTF-8",
  "LANG=C.UTF-8","TMPDIR=/tmp","HOME=/nonexistent","TZ=UTC","SOURCE_DATE_EPOCH=0",0};
static int ui_exit_status(int status) {
  require((status&127)!=127,"PLAYGROUND-UI-CHILD-STATE\n");
  return (status&127)?128+(status&127):((status>>8)&255);
}
static void ui_wait(long child) {
  int code=-1;unsigned long teardown=0;
  for(;;) {
    int status=0;long result=call6(61,-1,(long)&status,1,0,0,0);
    if(result==child) {
      code=ui_exit_status(status);teardown=monotonic_ms()+2000;
      long killed=sc(62,-1,9,0);
      require(killed==0 || killed==-3,"PLAYGROUND-UI-CHILD-TEARDOWN\n");
    } else if(result==-10) {
      require(code>=0,"PLAYGROUND-UI-CHILD-STATUS\n");stop(code);
    } else require(result>=0 || result==-4,"PLAYGROUND-UI-REAP\n");
    if(code>=0)require(monotonic_ms()<teardown,"PLAYGROUND-UI-REAP-DEADLINE\n");
    if(result<=0)pause_tick();
  }
}
void ui_start(unsigned long *stack) {
  close_inherited();verify_process_policy();
  require(stack[0]==1 && same(((char **)(stack+1))[0],"/ui-supervisor"),"PLAYGROUND-UI-MODE\n");
  kernel_watchdog();
  /* This separate, pretrusted test role has the explicitly reviewed 300s lifetime. */
  long previous=sc(37,300,0,0);
  require(previous>0 && previous<=12,"PLAYGROUND-UI-WATCHDOG\n");
  ui_role();long child=sc(57,0,0,0);
  require(child>=0,"PLAYGROUND-UI-FORK\n");
  if(!child) {
    require(sc(109,0,0,0)==0,"PLAYGROUND-UI-PROCESS-GROUP\n");
    close_inherited();
    char *const arguments[]={"/materials/runtime/node/bin/node","/app/ui-driver.mjs",0};
    sc(59,(long)arguments[0],(long)arguments,(long)ui_environment);
    fail("PLAYGROUND-UI-EXEC\n");
  }
  ui_wait(child);
}
__asm__(".global _start\n.type _start,@function\n_start:\nmov %rsp,%rdi\nandq $-16,%rsp\ncall ui_start\nud2\n.size _start,.-_start\n");
