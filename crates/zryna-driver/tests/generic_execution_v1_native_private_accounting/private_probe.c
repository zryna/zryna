/* Test-only private ABI, same unchanged ordinary object; qualified SystemV/x86_64. */
#include "observer_base.c"
#include "private_binding.h"
static uintptr_t address(int64_t delta) {
  uintptr_t base=(uintptr_t)&zryna_v1_e_entry;
  if(delta>=0) {assert(base<=UINTPTR_MAX-(uint64_t)delta);return base+(uint64_t)delta;}
  uint64_t magnitude=(uint64_t)(-(delta+1))+1;
  assert(base>=magnitude);return base-magnitude;
}
static void record_private(uint64_t phase,uint32_t status,uint64_t out,const uint64_t *context) {
  const uint64_t row[18]={phase,status,out,context[0],context[1],context[2],context[3],allocation_fault,
    summary[FROM],summary[CLONE],summary[RELEASE],summary[ATTEMPTS],summary[ALLOCS],summary[FREES],
    summary[LIVE],summary[OWNERS],summary[HIGH],summary[ZERO]};
  const uint8_t *bytes=(const uint8_t *)row;size_t left=sizeof(row);
  while(left) {ssize_t n=write(trace_fd,bytes,left);if(n<0 && errno==EINTR) continue;assert(n>0);bytes+=(size_t)n;left-=(size_t)n;}
}
int main(int argc,char **argv) {
  assert(argc==7);
  uint64_t context[4];for(size_t i=0;i<4;i++) context[i]=number(argv[i+1],UINT64_MAX);
  unsigned fault=(unsigned)number(argv[5],1);
  trace_fd=open(argv[6],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  uint64_t out=0;
#if M416_PRIVATE_BORROW
  zryna_rt_o1_handle parent={0};
  assert(__wrap_zryna_rt_o1_string_from_utf8_copy((const uint8_t *)"x",1,&parent)==0);
#endif
  allocation_fault=fault;record_private(0,0,0,context);
#if M416_PRIVATE_BORROW
  typedef uint32_t (*step_type)(uint64_t *,const zryna_rt_o1_handle *,uint64_t *);
  uint32_t status=((step_type)address(M416_STEP_DELTA))(&out,&parent,context);
#else
  typedef uint32_t (*step_type)(uint64_t *,uint64_t *);
  uint32_t status=((step_type)address(M416_STEP_DELTA))(&out,context);
#endif
  record_private(1,status,out,context);
#if M416_PRIVATE_BORROW
  assert(status!=0);out=0;
  typedef uint32_t (*consume_type)(uint64_t *,const zryna_rt_o1_handle *,uint64_t *);
  uint32_t cleanup=((consume_type)address(M416_CONSUME_DELTA))(&out,&parent,context);
  record_private(2,cleanup,out,context);
#endif
  assert(close(trace_fd)==0);return 0;
}
