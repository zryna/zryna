/* Fixed-array test observer; no caller source, runtime grant or observer heap. */
#define zryna_rt_o1_allocate __real_zryna_rt_o1_allocate
#define zryna_rt_o1_grow __real_zryna_rt_o1_grow
#define zryna_rt_o1_release __real_zryna_rt_o1_release
#define zryna_rt_o1_string_from_utf8_copy __real_zryna_rt_o1_string_from_utf8_copy
#define zryna_rt_o1_string_clone __real_zryna_rt_o1_string_clone
#define zryna_rt_o1_string_concat __real_zryna_rt_o1_string_concat
#define zryna_rt_o1_string_release __real_zryna_rt_o1_string_release
#define zryna_rt_o1_vec_allocate __real_zryna_rt_o1_vec_allocate
#define zryna_rt_o1_vec_reserve __real_zryna_rt_o1_vec_reserve
#define zryna_rt_o1_vec_release_storage __real_zryna_rt_o1_vec_release_storage
#define zryna_rt_o1_strong_clone __real_zryna_rt_o1_strong_clone
#define zryna_rt_o1_weak_downgrade __real_zryna_rt_o1_weak_downgrade
#define zryna_rt_o1_weak_clone __real_zryna_rt_o1_weak_clone
#define zryna_rt_o1_weak_upgrade __real_zryna_rt_o1_weak_upgrade
#define zryna_rt_o1_strong_release_begin __real_zryna_rt_o1_strong_release_begin
#define zryna_rt_o1_strong_release_finish __real_zryna_rt_o1_strong_release_finish
#define zryna_rt_o1_weak_release __real_zryna_rt_o1_weak_release
#include "zryna_ownership_runtime_v1.h"
#undef zryna_rt_o1_allocate
#undef zryna_rt_o1_grow
#undef zryna_rt_o1_release
#undef zryna_rt_o1_string_from_utf8_copy
#undef zryna_rt_o1_string_clone
#undef zryna_rt_o1_string_concat
#undef zryna_rt_o1_string_release
#undef zryna_rt_o1_vec_allocate
#undef zryna_rt_o1_vec_reserve
#undef zryna_rt_o1_vec_release_storage
#undef zryna_rt_o1_strong_clone
#undef zryna_rt_o1_weak_downgrade
#undef zryna_rt_o1_weak_clone
#undef zryna_rt_o1_weak_upgrade
#undef zryna_rt_o1_strong_release_begin
#undef zryna_rt_o1_strong_release_finish
#undef zryna_rt_o1_weak_release
#include "runtime_metadata.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <unistd.h>
typedef struct {uintptr_t base,pointer;uint64_t bytes,length,capacity;unsigned kind,live,owner;} block;
static block blocks[6];
static unsigned allocated,owners,inside_runtime,fault_code,from_calls,faults;
static int trace_fd;
static size_t trace_bytes;
static void record(uint64_t kind,uint64_t a,uint64_t b,uint64_t c,uint64_t e,uint64_t f,uint64_t g,uint64_t h) {
  uint64_t row[8]={kind,a,b,c,e,f,g,h};assert(trace_bytes<=65536-sizeof(row));
  const uint8_t *bytes=(const uint8_t *)row;size_t left=sizeof(row);
  while(left) {ssize_t n=write(trace_fd,bytes,left);if(n<0 && errno==EINTR) continue;assert(n>0);bytes+=(size_t)n;left-=(size_t)n;}
  trace_bytes+=sizeof(row);
}
static uint64_t payload(uintptr_t pointer,uint64_t length) {
  assert(length<=8);uint64_t result=0;
  for(uint64_t i=0;i<length;i++) result|=(uint64_t)((const uint8_t *)pointer)[i]<<(8*i);
  return result;
}
static block *find(uintptr_t pointer,unsigned kind) {
  for(unsigned i=0;i<allocated;i++) if(blocks[i].pointer==pointer && blocks[i].live && blocks[i].owner) {
    assert(blocks[i].kind==kind);return &blocks[i];
  }
  assert(!"invalid/interior/foreign/released owner");return NULL;
}
static void enter(unsigned role) {assert(!inside_runtime);record(4,role,0,0,0,0,0,0);inside_runtime=1;}
static uint32_t leave(unsigned role,uint32_t status) {assert(inside_runtime==1);inside_runtime=0;record(5,role,status,0,0,0,0,0);return status;}
static void zero(const zryna_rt_o1_handle *out) {assert(out && !out->pointer && !out->length && !out->capacity);}
static void acquire(uintptr_t pointer,uint64_t capacity,uint64_t length,uint64_t stride,unsigned kind,unsigned source) {
  block *chosen=NULL;assert(pointer && length<=capacity && stride && capacity<=UINT64_MAX/stride);
  for(unsigned i=0;i<allocated;i++) if(blocks[i].live && !blocks[i].owner && pointer>=blocks[i].base && pointer-blocks[i].base<=blocks[i].bytes && capacity*stride<=blocks[i].bytes-(pointer-blocks[i].base)) {assert(!chosen);chosen=&blocks[i];}
  assert(chosen);chosen->pointer=pointer;chosen->length=length;chosen->capacity=capacity;chosen->kind=kind;chosen->owner=++owners;
  record(2,owners,kind,pointer,chosen->base,source,length,kind==1?payload(pointer,length):0);
}
static void immutable(const zryna_rt_o1_handle *value,const zryna_rt_o1_handle *before,uint64_t bytes) {
  assert(memcmp(value,before,sizeof(*value))==0 && payload(value->pointer,value->length)==bytes);
}
