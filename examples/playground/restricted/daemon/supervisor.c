/* Trusted fixed playground process supervisor; no source-selected executable or argument. */
#include "supervisor_sys.h"
#include "supervisor_files.h"
#include "supervisor_pump.h"

static char *const environment[]={"PATH=/materials/runtime/node/bin","LC_ALL=C.UTF-8",
  "LANG=C.UTF-8","TMPDIR=/tmp","HOME=/nonexistent","TZ=UTC","SOURCE_DATE_EPOCH=0",0};
__attribute__((noreturn)) static void execute_fixed(int evaluate) {
  char *const compile_arguments[]={"/app/compiler","--materials-root","/materials",0};
  char *const evaluate_arguments[]={"/materials/runtime/node/bin/node","/app/evaluate.mjs",0};
  char *const *arguments=evaluate?evaluate_arguments:compile_arguments;
  sc(59,(long)arguments[0],(long)arguments,(long)environment);
  fail("PLAYGROUND-HOST-EXEC\n");
}
static void create_pipe(int descriptors[2]) {
  require(sc(293,(long)descriptors,0x80000,0)==0,"PLAYGROUND-HOST-PIPE\n");
  long flags=sc(72,descriptors[0],3,0);
  require(flags>=0 && sc(72,descriptors[0],4,flags|0x800)==0,"PLAYGROUND-HOST-PIPE\n");
}
static void run_job(int evaluate) {
  require(sc(39,0,0,0)>1,"PLAYGROUND-HOST-EXEC-PID\n");
  unsigned long deadline=monotonic_ms()+(evaluate?5000:10000);
  long input=verified_input(evaluate?2097152:32768);
  long root=directory("/output",65534,0700);directory_members(root,0);
  long stdout_file=output_file(root,"stdout"),stderr_file=output_file(root,"stderr");
  close_fd(root);
  int stdout_pipe[2]={-1,-1},stderr_pipe[2]={-1,-1};
  create_pipe(stdout_pipe);create_pipe(stderr_pipe);
  long child=sc(57,0,0,0);
  require(child>=0,"PLAYGROUND-HOST-FORK\n");
  if(!child) {
    require(sc(109,0,0,0)==0,"PLAYGROUND-HOST-PROCESS-GROUP\n");
    require(sc(33,input,0,0)==0 && sc(33,stdout_pipe[1],1,0)==1
      && sc(33,stderr_pipe[1],2,0)==2,"PLAYGROUND-HOST-PIPE\n");
    close_inherited();execute_fixed(evaluate);
  }
  close_fd(input);close_fd(stdout_pipe[1]);close_fd(stderr_pipe[1]);
  struct stream streams[2]={{stdout_pipe[0],stdout_file,0,evaluate?16384:1277956,1},
    {stderr_pipe[0],stderr_file,0,4096,1}};
  stop(collect(child,streams,deadline));
}
static void audit_materials(void) {
  require(sc(39,0,0,0)>1,"PLAYGROUND-HOST-EXEC-PID\n");
  long fd=sc(257,-100,(long)"/dev/null",0xa0000);
  require(fd>2,"PLAYGROUND-HOST-AUDIT-STDIN\n");
  struct file_status value={0};file_stat(fd,&value);
  require(value.uid==0 && value.gid==0 && (value.mode&0170000)==0020000
    && value.rdev==0x103,"PLAYGROUND-HOST-AUDIT-STDIN\n");
  require(sc(33,fd,0,0)==0,"PLAYGROUND-HOST-AUDIT-STDIN\n");
  close_fd(fd);close_inherited();
  char *const arguments[]={"/app/compiler","--audit-materials",0};
  sc(59,(long)arguments[0],(long)arguments,(long)environment);
  fail("PLAYGROUND-HOST-AUDIT-EXEC\n");
}
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
  require(argc==2,"PLAYGROUND-HOST-MODE\n");
  if(same(argv[1],"idle")){idle();}
  if(same(argv[1],"audit")){audit_materials();}
  if(same(argv[1],"compile")){run_job(0);}
  if(same(argv[1],"evaluate")){run_job(1);}
  fail("PLAYGROUND-HOST-MODE\n");
}
__asm__(".global _start\n.type _start,@function\n_start:\nmov %rsp,%rdi\nandq $-16,%rsp\ncall supervisor_start\nud2\n.size _start,.-_start\n");
