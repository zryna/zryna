/* Trusted CLI-only PID1: no executable, source, mode or environment selectors. */
#include "cli-supervisor-sys.h"
static void idle(void) {
  require(sc(39,0,0,0)==1,"PLAYGROUND-HOST-INIT-PID\n");
  unsigned long deadline=monotonic_ms()+12000;
  for(;;) {
    if(monotonic_ms()>=deadline)stop(124);
    for(unsigned count=0;count<256;++count) {
      int status=0;long child=call6(61,-1,(long)&status,1,0,0,0);
      if(child==0 || child==-10)break;
      require(child>0 || child==-4,"PLAYGROUND-HOST-INIT-REAP\n");
    }
    pause_tick();
  }
}
void supervisor_start(unsigned long *stack) {
  unsigned long argc=stack[0];char **argv=(char **)(stack+1);
  close_inherited();verify_process_policy();kernel_watchdog();
  require(argc==2 && same(argv[1],"idle"),"PLAYGROUND-HOST-MODE\n");
  idle();
}
__asm__(".global _start\n.type _start,@function\n_start:\nmov %rsp,%rdi\nandq $-16,%rsp\ncall supervisor_start\nud2\n.size _start,.-_start\n");
