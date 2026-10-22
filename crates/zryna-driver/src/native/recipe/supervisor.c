/* Trusted static Linux x86-64 recipe supervisor; never a fixture output. */
_Static_assert(sizeof(long)==8 && sizeof(void *)==8,"Linux x86-64 LP64 only");
static long call6(long n,long a,long b,long c,long d,long e,long f) {
  register long r10 __asm__("r10")=d;
  register long r8 __asm__("r8")=e;
  register long r9 __asm__("r9")=f;
  long result;
  __asm__ volatile("syscall":"=a"(result):"a"(n),"D"(a),"S"(b),"d"(c),
                   "r"(r10),"r"(r8),"r"(r9):"rcx","r11","memory","cc");
  return result;
}
static long sc(long n,long a,long b,long c) {return call6(n,a,b,c,0,0,0);}
static int same(const char *a,const char *b) {
  for(;*a && *a==*b;++a,++b) {}
  return *a==*b;
}
static unsigned long length(const char *text) {
  unsigned long n=0;while(text[n])++n;return n;
}
__attribute__((noreturn)) static void stop(int code) {sc(231,code,0,0);for(;;){}}
__attribute__((noreturn)) static void fail(const char *message) {
  sc(1,2,(long)message,(long)length(message));stop(70);
}
static void require(int condition,const char *message) {if(!condition)fail(message);}
static void close_inherited(void) {
  require(sc(436,3,0xffffffffUL,0)==0,"close_range failed; inherited FDs unproved\n");
}
struct file_status {
  unsigned long dev,ino,nlink;unsigned mode,uid,gid,pad;unsigned long rdev;
  long size,blksize,blocks,atime,atime_nsec,mtime,mtime_nsec,ctime,ctime_nsec,unused[3];
};
_Static_assert(sizeof(struct file_status)==144,"kernel stat size");
_Static_assert(__builtin_offsetof(struct file_status,mode)==24,"kernel stat offset");
static void file_stat(long fd,struct file_status *status) {
  require(sc(5,fd,(long)status,0)==0,"fstat failed\n");
}
static void close_fd(long fd) {require(sc(3,fd,0,0)==0,"close failed\n");}
static void mount_flags(const char *path,long flags,long magic) {
  long data[15]={0};
  require(sc(137,(long)path,(long)data,0)==0,"statfs failed\n");
  require((data[10]&flags)==flags,"required mount flags absent\n");
  require(!magic || data[0]==magic,"unexpected mount filesystem\n");
}
static long control_directory(void) {
  mount_flags("/control",14,0x01021994);
  long fd=sc(257,-100,(long)"/control",0xb0000);
  require(fd>=0,"control directory open failed\n");
  struct file_status status={0};file_stat(fd,&status);
  require(status.uid==0 && status.gid==0 && (status.mode&0177777)==0040555,
          "control directory ownership/mode mismatch\n");
  return fd;
}
static void numbered_path(char *out,const char *prefix,long number,const char *suffix) {
  require(number>=0 && number<=2147483647,"bounded path number required\n");
  unsigned used=0;while(*prefix)out[used++]=*prefix++;
  char digits[10];unsigned count=0;
  do{digits[count++]=(char)('0'+number%10);number/=10;}while(number);
  while(count)out[used++]=digits[--count];
  while(*suffix)out[used++]=*suffix++;
  out[used]=0;
}
static void readonly_mem(const char *path) {
  long fd=sc(257,-100,(long)path,0xa0000);
  require(fd>=0,"proc memory read-only open failed\n");close_fd(fd);
  require(sc(257,-100,(long)path,0xa0001)==-30,"proc memory WRONLY not EROFS\n");
  require(sc(257,-100,(long)path,0xa0002)==-30,"proc memory RDWR not EROFS\n");
}
static void inspect(void) {
  mount_flags("/",1,0);
  const char *paths[]={"/input","/tmp","/output","/dev","/dev/shm"};
  for(unsigned i=0;i<5;++i)mount_flags(paths[i],8,0);
  mount_flags("/proc",9,0x9fa0);
  long control=control_directory();close_fd(control);
  long directory=sc(257,-100,(long)"/input",0xb0000);
  require(directory>=0,"input directory open failed\n");
  struct file_status root={0};file_stat(directory,&root);
  require(root.uid==0 && root.gid==0 && (root.mode&0177777)==0040555,
          "input directory ownership/mode mismatch\n");
  const char *files[]={"library.c","library.h","sqlite3.h"};
  for(unsigned i=0;i<3;++i) {
    long fd=sc(257,directory,(long)files[i],0xa0800);
    require(fd>=0,"input open failed\n");
    struct file_status status={0};file_stat(fd,&status);
    require(status.uid==0 && status.gid==0 && status.nlink==1
      && status.size>0 && status.size<=2097152 && (status.mode&0177777)==0100444,
      "input ownership/mode/size mismatch\n");
    close_fd(fd);
  }
  close_fd(directory);
  readonly_mem("/proc/self/mem");readonly_mem("/proc/thread-self/mem");
  char task[64];numbered_path(task,"/proc/self/task/",sc(186,0,0,0),"/mem");
  readonly_mem(task);readonly_mem("/proc/self/root/proc/self/mem");
  const char line[]={"{\"kernelMountFlags\":true,\"inputModes\":true,"
    "\"procMemoryReadOnly\":true,\"controlModes\":true,\"inheritedFdsClosed\":true}\n"};
  require(sc(1,1,(long)line,sizeof(line)-1)==(long)(sizeof(line)-1),"diagnostic write failed\n");
}
static void denied(long result,const char *message) {
  require(result==-1 || result==-13,message);
}
static void check_wx(void) {
  denied(sc(319,(long)"forbidden",0,0),"memfd_create not denied\n");
  denied(call6(9,0,4096,5,0x22,-1,0),"anonymous executable mmap not denied\n");
  denied(call6(9,0,4096,7,0x22,-1,0),"anonymous WX mmap not denied\n");
  long memory=call6(9,0,4096,3,0x22,-1,0);
  require(memory>0,"ordinary anonymous allocation failed\n");
  denied(sc(10,memory,4096,5),"executable mprotect not denied\n");
  denied(call6(329,memory,4096,5,-1,0,0),"executable pkey_mprotect not denied\n");
  require(sc(11,memory,4096,0)==0,"control munmap failed\n");
  denied(sc(135,0x400000,0,0),"READ_IMPLIES_EXEC personality not denied\n");
  denied(call6(30,0,0,0100000,0,0,0),"SHM_EXEC shmat not denied\n");
  denied(sc(101,0,0,0),"ptrace not denied\n");
  denied(call6(311,sc(39,0,0,0),0,0,0,0,0),"process_vm_writev not denied\n");
  const char line[]="{\"explicitMemoryDenials\":true}\n";
  require(sc(1,1,(long)line,sizeof(line)-1)==(long)(sizeof(line)-1),"control write failed\n");
}
static void check_boundaries(void) {
  const long domains[]={2,10,1};int pair[2]={-1,-1};
  for(unsigned i=0;i<3;++i)
    require(sc(41,domains[i],1,0)==-1,"socket boundary not EPERM\n");
  require(call6(53,1,1,0,(long)pair,0,0)==-1,"socketpair boundary not EPERM\n");
  require(sc(272,0x10000000,0,0)==-1,"unshare boundary not EPERM\n");
  /* Follow this fixed namespace alias intentionally; setns must be denied on a valid fd. */
  long ns=sc(257,-100,(long)"/proc/self/ns/mnt",0x80000);
  require(ns>=0,"namespace sensitivity open failed\n");
  require(sc(308,ns,0x20000,0)==-1,"setns boundary not EPERM\n");close_fd(ns);
  require(call6(165,(long)"tmpfs",(long)"/tmp",(long)"tmpfs",0,
                (long)"size=4096",0)==-1,"mount boundary not EPERM\n");
  mount_flags("/input",1,0);mount_flags("/",1,0);mount_flags("/control",1,0);
  const char *empty_files[]={"/etc/hostname","/etc/hosts","/etc/resolv.conf"};
  for(unsigned i=0;i<3;++i) {
    long masked=sc(257,-100,(long)empty_files[i],0xa0000);
    require(masked>=0,"masked /etc sensitivity open failed\n");
    struct file_status status={0};file_stat(masked,&status);
    require(status.uid==0 && status.gid==0 && (status.mode&0170000)==0020000
      && status.rdev==0x103,"masked /etc null-device identity differs\n");
    char byte=0;require(sc(0,masked,(long)&byte,1)==0,"masked /etc read not EOF\n");
    close_fd(masked);
  }
  const char *inputs[]={"/input/library.c","/input/library.h","/input/sqlite3.h"};
  for(unsigned i=0;i<3;++i) {
    long fd=sc(257,-100,(long)inputs[i],0xa0000);
    require(fd>=0,"readonly input sensitivity open failed\n");close_fd(fd);
    long writing=sc(257,-100,(long)inputs[i],0xa0001);
    require(writing==-13 || writing==-30,"readonly input write not denied\n");
  }
  long root_write=call6(257,-100,(long)"/forbidden-boundary",0xa00c1,0600,0,0);
  require(root_write==-13 || root_write==-30,"readonly root create not denied\n");
  long control_write=call6(257,-100,(long)"/control/forbidden-boundary",0xa00c1,0600,0,0);
  require(control_write==-13 || control_write==-30,"readonly control create not denied\n");
  const char *path="/tmp/supervisor-boundary-control";
  long fd=call6(257,-100,(long)path,0xa00c2,0600,0,0);
  require(fd>=0,"private scratch create failed\n");char sent='q',got=0;
  require(sc(1,fd,(long)&sent,1)==1 && sc(8,fd,0,0)==0
    && sc(0,fd,(long)&got,1)==1 && got==sent,"scratch read/write failed\n");
  close_fd(fd);require(sc(263,-100,(long)path,0)==0,"scratch unlink failed\n");
  const char line[]="{\"declaredBoundaryDenials\":true,\"scratchWriteReadAllowed\":true}\n";
  require(sc(1,1,(long)line,sizeof(line)-1)==(long)(sizeof(line)-1),"boundary write failed\n");
}
static void orphan_control(void) {
  long child=sc(57,0,0,0);require(child>=0,"orphan fork sensitivity failed\n");
  if(child==0) {
    /* Do not keep Engine's attached streams open after the control parent exits. */
    sc(3,0,0,0);sc(3,1,0,0);sc(3,2,0,0);
    unsigned long started[2]={0},current[2]={0};const long tick[2]={0,100000000};
    require(sc(228,1,(long)started,0)==0 && started[0]<(~0UL)-31,
            "orphan monotonic clock unavailable\n");
    const unsigned long deadline=started[0]+30;
    for(;;) {
      require(sc(228,1,(long)current,0)==0,"orphan clock failed\n");
      if(current[0]>deadline || (current[0]==deadline && current[1]>=started[1]))stop(124);
      long slept=sc(35,(long)tick,0,0);require(slept==0 || slept==-4,"orphan sleep failed\n");
    }
  }
  char line[64];numbered_path(line,"{\"orphanNamespacePid\":",child,"}\n");
  unsigned long size=length(line);
  require(sc(1,1,(long)line,(long)size)==(long)size,"orphan identity write failed\n");
  stop(0);
}
static char *const environment[]={"PATH=/tool/bin:/tool/libexec","LC_ALL=C","LANG=C",
    "LD_LIBRARY_PATH=/lib/x86_64-linux-gnu:/lib64","TMPDIR=/tmp","HOME=/nonexistent",
    "TZ=UTC","SOURCE_DATE_EPOCH=0",0};
static void compile(void) {
  char *const arguments[]={"/tool/bin/gcc","-B/tool/libexec/","-B/tool/bin/",
    "-nostdinc","-isystem","/tool/include/gcc","-isystem","/tool/include/libc",
    "-isystem","/tool/include/arch","-std=c11","-O0","-g0","-fno-ident",
    "-fno-pie","-fno-pic","-fcf-protection=none","-fno-unwind-tables",
    "-fno-asynchronous-unwind-tables","-frandom-seed=zryna-sqlite-closed-v0",
    "-Wall","-Wextra","-Werror","-pedantic","-c","/input/library.c",
    "-o","/output/adapter.o",0};
  sc(59,(long)arguments[0],(long)arguments,(long)environment);
  fail("fixed GCC exec failed\n");
}
#include "supervisor_alias.h"
static void alias_rx_denied(const char *path,const struct file_status *original) {
  /* The descriptor alias is deliberately followed, then bound to the held inode. */
  long fd=sc(257,-100,(long)path,0x80000);
  require(fd>=0,"sensitivity alias reopen failed\n");
  struct file_status status={0};file_stat(fd,&status);
  require(status.dev==original->dev && status.ino==original->ino
    && status.uid==original->uid && status.gid==original->gid && status.nlink==1
    && status.size==original->size && (status.mode&0177777)==0100555,
    "sensitivity alias identity mismatch\n");
  require(call6(9,0,4096,5,2,fd,0)==-1,"alias RX mmap not EPERM\n");
  close_fd(fd);
}
static void check_alias(void) {
  long root=sc(257,-100,(long)"/tool/bin/gcc",0xa0000);
  require(root>=0,"approved GCC control open failed\n");
  long mapped=call6(9,0,4096,5,2,root,0);
  require(mapped>0,"approved GCC RX mmap failed\n");
  require(sc(11,mapped,4096,0)==0,"approved GCC munmap failed\n");close_fd(root);
  copy_alias_payloads();
  const char *areas[]={"/input/forbidden.elf","/tmp/forbidden.elf","/output/forbidden.elf"};
  const char *aliases[]={"/proc/self/root/input/forbidden.elf", "/proc/self/root/tmp/forbidden.elf",
    "/proc/self/root/output/forbidden.elf"};
  for(unsigned i=0;i<3;++i) {
    long fd=sc(257,-100,(long)areas[i],0xa0000);
    require(fd>=0,"noexec sensitivity payload open failed\n");
    struct file_status status={0};file_stat(fd,&status);
    unsigned owner=i==0?0:65534;
    require(status.uid==owner && status.gid==owner && status.nlink==1
      && status.size>=4096 && status.size<=65536 && (status.mode&0177777)==0100555,
      "fixed executable ELF sensitivity identity differs\n");
    unsigned char header[64]={0};
    require(sc(0,fd,(long)header,64)==64 && header[0]==127 && header[1]=='E'
      && header[2]=='L' && header[3]=='F' && header[4]==2 && header[5]==1
      && header[6]==1 && (header[16]==2 || header[16]==3) && header[17]==0
      && header[18]==62 && header[19]==0,"ELF64 executable sensitivity required\n");
    require(call6(9,0,4096,5,2,fd,0)==-1,"noexec data RX mmap not EPERM\n");
    char descriptor[64];numbered_path(descriptor,"/proc/self/fd/",fd,"");
    alias_rx_denied(descriptor,&status);alias_rx_denied(aliases[i],&status);
    char *const arguments[]={descriptor,"--version",0};
    require(sc(59,(long)descriptor,(long)arguments,(long)environment)==-13,
            "descriptor alias exec not EACCES\n");
    char *const rooted[]={ (char *)aliases[i],"--version",0};
    require(sc(59,(long)aliases[i],(long)rooted,(long)environment)==-13,
            "root alias exec not EACCES\n");
    close_fd(fd);
  }
  const char line[]={"{\"noexecAliasesDenied\":true,\"approvedToolRxAllowed\":true}\n"};
  require(sc(1,1,(long)line,sizeof(line)-1)==(long)(sizeof(line)-1),"alias control write failed\n");
}
static void idle(void) {
  require(sc(39,0,0,0)==1,"idle requires private namespace PID1\n");
  long directory=control_directory();close_fd(directory);
  const long pause_time[2]={0,100000000};
  unsigned long started[2]={0},current[2]={0};
  require(sc(228,1,(long)started,0)==0 && started[0]<(~0UL)-31,
          "watchdog monotonic clock unavailable\n");
  const unsigned long deadline=started[0]+30;
  for(;;) {
    require(sc(228,1,(long)current,0)==0,"watchdog clock failed\n");
    if(current[0]>deadline || (current[0]==deadline && current[1]>=started[1]))stop(124);
    for(unsigned reaped=0;reaped<16;++reaped) {
      long child=call6(61,-1,0,1,0,0,0);
      if(child==0 || child==-10)break;
      require(child>0 || child==-4,"bounded child reap failed\n");
    }
    long slept=sc(35,(long)pause_time,0,0);
    require(slept==0 || slept==-4,"watchdog sleep failed\n");
  }
}
void supervisor_start(unsigned long *stack) {
  unsigned long argc=stack[0];char **argv=(char **)(stack+1);
  close_inherited();
  require(sc(102,0,0,0)==65534 && sc(107,0,0,0)==65534
    && sc(104,0,0,0)==65534 && sc(108,0,0,0)==65534,"fixed nonroot UID/GID required\n");
  require(argc==2,"exactly one fixed supervisor mode required\n");
  if(same(argv[1],"idle")){idle();}
  if(same(argv[1],"inspect")){inspect();stop(0);}
  if(same(argv[1],"check-wx")){check_wx();stop(0);}
  if(same(argv[1],"check-boundaries")){check_boundaries();stop(0);}
  if(same(argv[1],"orphan-control")){orphan_control();}
  if(same(argv[1],"check-alias")){check_alias();stop(0);}
  if(same(argv[1],"compile")){compile();}
  fail("unsupported supervisor mode\n");
}
__asm__(".global _start\n.type _start,@function\n_start:\nmov %rsp,%rdi\nandq $-16,%rsp\ncall supervisor_start\nud2\n.size _start,.-_start\n");
