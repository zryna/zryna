#define _POSIX_C_SOURCE 200809L
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <unistd.h>
#include "zryna_ownership_runtime_v1.h"
#include "runtime_metadata.h"
extern int32_t zryna_v1_e_entry(int32_t);
extern void *__real_malloc(size_t);
extern void __real_free(void *);
extern uint32_t __real_zryna_rt_o1_allocate(uint64_t,uint32_t,uintptr_t *);
extern uint32_t __real_zryna_rt_o1_strong_clone(uintptr_t);
extern uint32_t __real_zryna_rt_o1_weak_downgrade(uintptr_t);
extern uint32_t __real_zryna_rt_o1_weak_clone(uintptr_t);
extern uint32_t __real_zryna_rt_o1_weak_upgrade(uintptr_t);
extern uint32_t __real_zryna_rt_o1_strong_release_begin(uintptr_t,uint32_t *);
extern uint32_t __real_zryna_rt_o1_strong_release_finish(uintptr_t);
extern uint32_t __real_zryna_rt_o1_weak_release(uintptr_t,uint32_t *);
/* Fixed actual-observation ABI:22 little-endian u64 fields, no event history. */
_Static_assert(ATOMIC_LONG_LOCK_FREE==2 && ATOMIC_LLONG_LOCK_FREE==2,"lock-free signal snapshot");
static _Atomic uint64_t summary[22];
static volatile sig_atomic_t trace_fd=-1;
static unsigned depth;
static uint64_t fault_ordinal;
static struct { void *base; size_t bytes; uintptr_t control; } block;
static unsigned pending_last;
enum { MAGIC,PHASE,INVOCATION,ALLOCATE,STRONGCLONE,DOWN,WEAKCLONE,UPGRADE,BEGIN,FINISH,WEAKRELEASE,
       ATTEMPTS,ALLOCS,FREES,LIVE,HIGH,STRONG,WEAK,STATUS,ZERO,EXPIRED,ALLOCATION_FAILURE };
static void write_summary(void) {
  uint64_t record[22];
  for(size_t i=0;i<22;i++) record[i]=atomic_load_explicit(&summary[i],memory_order_relaxed);
  const uint8_t *bytes=(const uint8_t *)record;size_t left=sizeof(record);
  while(left) {
    ssize_t n=write(trace_fd,bytes,left);
    if(n<0 && errno==EINTR) continue;
    if(n<=0) _exit(120);
    bytes+=(size_t)n;left-=(size_t)n;
  }
}
static void trapped(int number) {
  (void)number;summary[PHASE]=1;write_summary();
  /* SA_RESETHAND: retry actual trap under restored SIG_DFL, never a successful exit. */
}
void *__wrap_malloc(size_t bytes) {
  if(!depth) return __real_malloc(bytes);
  summary[ATTEMPTS]++;assert(bytes>0 && bytes<4096 && !block.base);
  if(fault_ordinal && summary[ATTEMPTS]==fault_ordinal) return NULL;
  void *pointer=__real_malloc(bytes);assert(pointer);
  block.base=pointer;block.bytes=bytes;summary[ALLOCS]++;summary[LIVE]++;
  if(summary[LIVE]>summary[HIGH]) summary[HIGH]=summary[LIVE];
  return pointer;
}
void __wrap_free(void *pointer) {
  if(!depth) { __real_free(pointer);return; }
  assert(pointer && pointer==block.base && block.control && summary[LIVE]==1);
  memset(&block,0,sizeof(block));summary[FREES]++;summary[LIVE]--;__real_free(pointer);
}
static void control(uintptr_t pointer) {
  assert(pointer && pointer==block.control && block.base && summary[LIVE]==1);
  const uint32_t *counts=(const uint32_t *)pointer;
  assert(counts[0]==summary[STRONG]);
  assert(counts[1]==summary[WEAK]+(summary[STRONG] || pending_last ? 1U : 0U));
  assert(*(const int32_t *)(pointer+8)==7);
}
static void event(size_t slot) { assert(!depth);summary[slot]++;summary[STATUS]++; }
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t size,uint32_t alignment,uintptr_t *out) {
  assert(out && !*out && !pending_last && !summary[STRONG] && !summary[WEAK]);
  assert(metadata_control(size,alignment) && size==12 && alignment==4);
  event(ALLOCATE);summary[ZERO]++;
  depth++;uint32_t status=__real_zryna_rt_o1_allocate(size,alignment,out);depth--;
  if(status==1) { assert(!*out && !block.base);summary[ALLOCATION_FAILURE]++;return status; }
  assert(status==0 && *out && block.base);
  uintptr_t base=(uintptr_t)block.base;
  assert(*out>=base && *out-base+size<=block.bytes);
  block.control=*out;summary[STRONG]=1;
  /* Source initializes real strong/implicitweak and payload after successful allocation. */
  return status;
}
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t pointer) {
  control(pointer);assert(summary[STRONG]>0 && !pending_last);event(STRONGCLONE);
  depth++;uint32_t status=__real_zryna_rt_o1_strong_clone(pointer);depth--;
  assert(status==0);summary[STRONG]++;control(pointer);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t pointer) {
  control(pointer);assert(summary[STRONG]>0 && !pending_last);event(DOWN);
  depth++;uint32_t status=__real_zryna_rt_o1_weak_downgrade(pointer);depth--;
  assert(status==0);summary[WEAK]++;control(pointer);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t pointer) {
  control(pointer);assert(summary[WEAK]>0 && !pending_last);event(WEAKCLONE);
  depth++;uint32_t status=__real_zryna_rt_o1_weak_clone(pointer);depth--;
  assert(status==0);summary[WEAK]++;control(pointer);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t pointer) {
  control(pointer);assert(summary[WEAK]>0 && !pending_last);event(UPGRADE);
  uint64_t strong=summary[STRONG];
  depth++;uint32_t status=__real_zryna_rt_o1_weak_upgrade(pointer);depth--;
  if(strong) { assert(status==0);summary[STRONG]++; }
  else { assert(status==5);summary[EXPIRED]++; }
  control(pointer);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t pointer,uint32_t *out) {
  control(pointer);assert(out && !*out && summary[STRONG]>0 && !pending_last);
  event(BEGIN);summary[ZERO]++;
  uint64_t strong=summary[STRONG];
  depth++;uint32_t status=__real_zryna_rt_o1_strong_release_begin(pointer,out);depth--;
  assert(status==0 && *out==(strong==1));summary[STRONG]--;pending_last=*out;
  control(pointer);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t pointer) {
  control(pointer);assert(!summary[STRONG] && pending_last);event(FINISH);
  uint64_t weak=summary[WEAK];
  depth++;uint32_t status=__real_zryna_rt_o1_strong_release_finish(pointer);depth--;
  assert(status==0);pending_last=0;
  if(!weak) { assert(!block.base && !summary[LIVE]); }
  else control(pointer);
  return status;
}
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t pointer,uint32_t *out) {
  control(pointer);assert(out && !*out && summary[WEAK]>0 && !pending_last);
  event(WEAKRELEASE);summary[ZERO]++;
  unsigned last=summary[WEAK]==1 && !summary[STRONG];
  depth++;uint32_t status=__real_zryna_rt_o1_weak_release(pointer,out);depth--;
  assert(status==0 && *out==last);summary[WEAK]--;
  if(last) { assert(!block.base && !summary[LIVE]); }
  else control(pointer);
  return status;
}
static uint64_t number(const char *text,uint64_t max) {
  char *end=NULL;errno=0;unsigned long long n=strtoull(text,&end,10);
  assert(!errno && end!=text && !*end && n<=max);return (uint64_t)n;
}
int main(int argc,char **argv) {
  (void)metadata_element;
  assert(argc==5);
  uint64_t trips=number(argv[1],1048577),repeats=number(argv[2],2);
  fault_ordinal=number(argv[4],1);assert(repeats>0);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  trace_fd=open(argv[3],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  struct sigaction action={0};action.sa_handler=trapped;action.sa_flags=SA_RESETHAND;
  assert(sigemptyset(&action.sa_mask)==0 && sigaction(SIGILL,&action,NULL)==0);
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);
  for(uint64_t invocation=1;invocation<=repeats;invocation++) {
    assert(!depth && !block.base && !pending_last && !summary[STRONG] && !summary[WEAK]);
    for(size_t i=0;i<22;i++) atomic_store_explicit(&summary[i],0,memory_order_relaxed);
    summary[MAGIC]=UINT64_C(0x4d34313641434331);summary[INVOCATION]=invocation;
    int32_t result=zryna_v1_e_entry((int32_t)trips);
    assert(!depth && !block.base && !pending_last && !summary[STRONG] && !summary[WEAK]);
    assert(summary[ALLOCS]==summary[FREES] && !summary[LIVE]);
    summary[PHASE]=2;write_summary();printf("result:%d\n",result);
  }
  assert(close(trace_fd)==0);return 0;
}
