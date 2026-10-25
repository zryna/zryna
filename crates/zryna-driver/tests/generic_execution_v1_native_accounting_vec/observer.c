#define _POSIX_C_SOURCE 200809L
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
extern int32_t zryna_v1_e_entry(int32_t);
extern void *__real_malloc(size_t);
extern void __real_free(void *);
#include "runtime_metadata.h"
extern uint32_t __real_zryna_rt_o1_vec_allocate(uint32_t,uint64_t,zryna_rt_o1_handle *);
extern uint32_t __real_zryna_rt_o1_vec_reserve(uint32_t,const zryna_rt_o1_handle *,uint64_t,zryna_rt_o1_handle *);
extern uint32_t __real_zryna_rt_o1_vec_release_storage(uint32_t,const zryna_rt_o1_handle *);
/* Fixed ABI: fourteen little-endian u64 fields, no per-event retained history. */
_Static_assert(ATOMIC_LONG_LOCK_FREE==2 && ATOMIC_LLONG_LOCK_FREE==2,"signal snapshot requires lock-free u64 atomics");
static _Atomic uint64_t summary[14];
static volatile sig_atomic_t trace_fd=-1;
static unsigned depth;
struct block { void *base; size_t bytes; uintptr_t payload; uint64_t capacity; };
static struct block blocks[2];
enum { MAGIC,PHASE,INVOCATION,ALLOCATE,RESERVE,RELEASE,ATTEMPTS,ALLOCS,FREES,LIVE,HIGH,OWNERS,STATUS,ZERO };
static void write_summary(void) {
  uint64_t record[14];
  for(size_t i=0;i<14;i++) record[i]=atomic_load_explicit(&summary[i],memory_order_relaxed);
  const uint8_t *bytes=(const uint8_t *)record;
  size_t left=sizeof(record);
  while(left) {
    ssize_t n=write(trace_fd,bytes,left);
    if(n<0 && errno==EINTR) continue;
    if(n<=0) _exit(120);
    bytes+=(size_t)n;left-=(size_t)n;
  }
}
static void trapped(int number) {
  (void)number;
  summary[PHASE]=1;
  write_summary();
  /* SA_RESETHAND restores SIG_DFL; returning retries the actual trap instruction. */
}
void *__wrap_malloc(size_t bytes) {
  if(!depth) return __real_malloc(bytes);
  summary[ATTEMPTS]++;
  assert(bytes>0 && bytes<4096);
  void *pointer=__real_malloc(bytes);assert(pointer);
  for(size_t i=0;i<2;i++) if(!blocks[i].base) {
    blocks[i]=(struct block){pointer,bytes,0,0};
    summary[ALLOCS]++;summary[LIVE]++;
    if(summary[LIVE]>summary[HIGH]) summary[HIGH]=summary[LIVE];
    return pointer;
  }
  assert(!"fixture has at most two simultaneous blocks");return NULL;
}
void __wrap_free(void *pointer) {
  if(!depth) { __real_free(pointer);return; }
  assert(pointer);
  for(size_t i=0;i<2;i++) if(blocks[i].base==pointer) {
    assert(blocks[i].payload && summary[LIVE]>0);
    blocks[i]=(struct block){0};summary[FREES]++;summary[LIVE]--;
    __real_free(pointer);return;
  }
  assert(!"release must use an actually live allocation base");
}
static void element(uint32_t id) {
  uint64_t stride=0;uint32_t alignment=0;
  assert(metadata_element(id,&stride,&alignment) && stride==4 && alignment==4);
}
static void validate(const zryna_rt_o1_handle *value) {
  assert(value && value->length<=value->capacity && value->length<=2 && value->capacity<=4);
  if(!value->capacity) { assert(!value->pointer && !value->length);return; }
  for(size_t i=0;i<2;i++) if(blocks[i].payload==value->pointer) {
    assert(blocks[i].capacity==value->capacity);
    const int32_t *data=(const int32_t *)value->pointer;
    for(uint64_t n=0;n<value->length;n++) assert(data[n]==(n==0?7:11));
    return;
  }
  assert(!"read or release must select a live Vec backing allocation");
}
static void zero(const zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);summary[ZERO]++;
}
static void backing(const zryna_rt_o1_handle *out) {
  assert(out->length<=out->capacity && out->capacity<=4);
  if(!out->capacity) { assert(!out->pointer);return; }
  for(size_t i=0;i<2;i++) {
    uintptr_t base=(uintptr_t)blocks[i].base;
    if(base && out->pointer>=base && out->pointer-base<blocks[i].bytes) {
      assert((out->pointer-base)+out->capacity*4<=blocks[i].bytes);
      assert(!blocks[i].payload || blocks[i].payload==out->pointer);
      blocks[i].payload=out->pointer;blocks[i].capacity=out->capacity;return;
    }
  }
  assert(!"returned Vec backing must refer to its actual canonical allocation");
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t id,uint64_t capacity,zryna_rt_o1_handle *out) {
  assert(!depth);element(id);zero(out);summary[ALLOCATE]++;summary[STATUS]++;
  depth++;uint32_t status=__real_zryna_rt_o1_vec_allocate(id,capacity,out);depth--;
  assert(status==0 && out->length==0 && out->capacity==capacity);
  backing(out);summary[OWNERS]++;return status;
}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t id,const zryna_rt_o1_handle *source,uint64_t required,zryna_rt_o1_handle *out) {
  assert(!depth && source!=out && summary[OWNERS]>0);element(id);validate(source);zero(out);
  zryna_rt_o1_handle before=*source;summary[RESERVE]++;summary[STATUS]++;
  depth++;uint32_t status=__real_zryna_rt_o1_vec_reserve(id,source,required,out);depth--;
  assert(status==0 && memcmp(source,&before,sizeof(before))==0);
  assert(out->length==before.length && out->capacity==4);
  if(before.capacity>=required) assert(out->pointer==before.pointer && out->capacity==before.capacity);
  backing(out);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t id,const zryna_rt_o1_handle *value) {
  assert(!depth && summary[OWNERS]>0);element(id);validate(value);
  summary[RELEASE]++;summary[STATUS]++;
  depth++;uint32_t status=__real_zryna_rt_o1_vec_release_storage(id,value);depth--;
  assert(status==0);summary[OWNERS]--;return status;
}
static uint64_t number(const char *text,uint64_t max) {
  char *end=NULL;errno=0;unsigned long long n=strtoull(text,&end,10);
  assert(!errno && end!=text && !*end && n<=max);return (uint64_t)n;
}
int main(int argc,char **argv) {
  (void)metadata_control;
  assert(argc==4);
  uint64_t trips=number(argv[1],2097153),repeats=number(argv[2],2);
  assert(repeats>0);
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  trace_fd=open(argv[3],O_WRONLY|O_CREAT|O_EXCL,0600);assert(trace_fd>=0);
  struct sigaction action={0};action.sa_handler=trapped;action.sa_flags=SA_RESETHAND;
  assert(sigemptyset(&action.sa_mask)==0 && sigaction(SIGILL,&action,NULL)==0);
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);
  for(uint64_t invocation=1;invocation<=repeats;invocation++) {
    assert(!depth && !summary[LIVE] && !summary[OWNERS]);
    for(size_t i=0;i<14;i++) atomic_store_explicit(&summary[i],0,memory_order_relaxed);
    summary[MAGIC]=UINT64_C(0x4d34313641434331);summary[INVOCATION]=invocation;
    int32_t result=zryna_v1_e_entry((int32_t)trips);
    assert(!depth && !summary[LIVE] && !summary[OWNERS] && summary[ALLOCS]==summary[FREES]);
    summary[PHASE]=2;write_summary();printf("result:%d\n",result);
  }
  assert(close(trace_fd)==0);return 0;
}
