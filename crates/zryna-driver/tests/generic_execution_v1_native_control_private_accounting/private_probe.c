/* Test-only projection of the unchanged ordinary private ABI with the exact existing C observer. */
#define main m416_unused_public_observer_main
#include "control_observer.c"
#undef main
#include "private_binding.h"
static uintptr_t private_address(int64_t delta) {
  uintptr_t base=(uintptr_t)&zryna_v1_e_entry;
  if(delta>=0) {assert(base<=UINTPTR_MAX-(uint64_t)delta);return base+(uint64_t)delta;}
  uint64_t magnitude=(uint64_t)(-(delta+1))+1;assert(base>=magnitude);return base-magnitude;
}
static void record_private(uint64_t phase,uint32_t status,uint64_t out,const uint64_t *context) {
  uint64_t row[30]={phase,status,out,context[0],context[1],context[2],context[3],
    fault_ordinal && summary[ATTEMPTS]<fault_ordinal};
  for(size_t i=0;i<22;i++) row[i+8]=atomic_load_explicit(&summary[i],memory_order_relaxed);
  const uint8_t *bytes=(const uint8_t *)row;size_t left=sizeof(row);
  while(left) {ssize_t n=write(trace_fd,bytes,left);if(n<0 && errno==EINTR)continue;assert(n>0);bytes+=(size_t)n;left-=(size_t)n;}
}
int main(int argc,char **argv) {
  assert(argc==7);uint64_t context[4];
  for(size_t i=0;i<4;i++)context[i]=number(argv[i+1],UINT64_MAX);
  fault_ordinal=number(argv[5],1);trace_fd=open(argv[6],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  summary[MAGIC]=UINT64_C(0x4d34313641434331);summary[INVOCATION]=1;
  uint64_t out=0;record_private(0,0,0,context);
  typedef uint32_t (*step_type)(uint64_t *,uint64_t *);
  uint32_t status=((step_type)private_address(M416_STEP_DELTA))(&out,context);
  record_private(1,status,out,context);
  assert(!depth && !block.base && !pending_last && !summary[STRONG] && !summary[WEAK]);
  assert(summary[ALLOCS]==summary[FREES] && !summary[LIVE]);
  assert(close(trace_fd)==0);return 0;
}
