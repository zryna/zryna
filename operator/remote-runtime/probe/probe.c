#define _GNU_SOURCE
/* Fixed trusted operator probes; no source input, shell, path or command selector. */
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static unsigned long long now_ns(void) {
  struct timespec value;
  if (clock_gettime(CLOCK_MONOTONIC, &value)) _exit(80);
  return (unsigned long long)value.tv_sec * 1000000000ULL + (unsigned long long)value.tv_nsec;
}
static void require(int value) { if (!value) _exit(81); }
static void reap(pid_t *children, unsigned count) {
  for (unsigned i=0; i<count; ++i) require(kill(children[i], SIGKILL)==0 || errno==ESRCH);
  for (unsigned i=0; i<count; ++i) { int status; require(waitpid(children[i], &status, 0)==children[i]); }
}
int main(int argc, char **argv) {
  require(argc==3 && getpid()==1 && geteuid()==65534 && getegid()==65534);
  int compile = strcmp(argv[1], "compiler")==0;
  require(compile || strcmp(argv[1], "evaluator")==0);
  unsigned long memory = compile ? 536870912UL : 1073741824UL;
  if (strcmp(argv[2], "policy")==0) {
    int fd=open("/proc/self/status", O_RDONLY|O_CLOEXEC); require(fd>=0);
    char buffer[16384]; ssize_t count=read(fd, buffer, sizeof(buffer)); require(count>0 && count<(ssize_t)sizeof(buffer));
    require(close(fd)==0); require(write(1,buffer,(size_t)count)==count);
    errno=0; long ptrace=syscall(SYS_ptrace,0,0,0,0); int pe=errno;
    errno=0; long bpf=syscall(SYS_bpf,0,NULL,0); int be=errno;
    errno=0; int root=open("/operator-write-probe",O_WRONLY|O_CREAT|O_EXCL,0600); int re=errno;
    if(root>=0) { close(root); unlink("/operator-write-probe"); }
    printf("PROBE uid=%u gid=%u ptrace=%ld errno=%d bpf=%ld errno=%d root=%d errno=%d\n",geteuid(),getegid(),ptrace,pe,bpf,be,root,re);
    require(ptrace==-1 && pe==EPERM && bpf==-1 && be==EPERM && root==-1 && re==EROFS);
  } else if (strcmp(argv[2], "scratch")==0) {
    size_t limit=compile ? 67108864UL : 134217728UL;
    int fd=open("/tmp/fixed-scratch-probe",O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC,0600); require(fd>=0);
    static char buffer[65536]; memset(buffer,0x5a,sizeof(buffer)); size_t total=0;
    for (;;) { ssize_t count=write(fd,buffer,sizeof(buffer)); if(count<0) { require(errno==ENOSPC); break; } require(count>0); total+=(size_t)count; require(total<=limit); }
    require(close(fd)==0 && unlink("/tmp/fixed-scratch-probe")==0 && total==limit);
    printf("PROBE scratch_bytes=%zu first_extra=ENOSPC\n",total);
  } else if (strcmp(argv[2], "tasks")==0) {
    pid_t children[256]; unsigned count=0, limit=compile ? 32 : 256; int error=0;
    while(count<256) { pid_t pid=fork(); if(pid<0) { error=errno; break; } if(!pid) { alarm(8); for(;;) pause(); } children[count++]=pid; }
    reap(children,count); printf("PROBE tasks=%u fork_errno=%d reaped=%u\n",count+1,error,count);
    require(count+1==limit && error==EAGAIN);
  } else if (strcmp(argv[2], "cpu")==0) {
    pid_t children[4]; unsigned long long end=now_ns()+2000000000ULL;
    for(unsigned i=0;i<4;++i) { pid_t pid=fork(); require(pid>=0); if(!pid) { volatile uint64_t x=1; while(now_ns()<end) for(unsigned j=0;j<10000;++j) x=x*6364136223846793005ULL+1; _exit(x==0 ? 1:0); } children[i]=pid; }
    for(unsigned i=0;i<4;++i) { int status; require(waitpid(children[i],&status,0)==children[i] && WIFEXITED(status) && WEXITSTATUS(status)==0); }
    puts("PROBE cpu_workers=4 duration_ns=2000000000 reaped=4");
  } else if (strcmp(argv[2], "memory")==0) {
    size_t size=memory+33554432UL; volatile unsigned char *value=mmap(NULL,size,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    require(value!=MAP_FAILED); printf("PROBE touching_bytes=%zu expected_memcg_oom=1\n",size); fflush(stdout);
    for(size_t i=0;i<size;i+=4096) value[i]=0x5a;
    require(munmap((void *)value,size)==0); return 82;
  } else return 83;
  return fflush(stdout)==0 ? 0 : 84;
}
