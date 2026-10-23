#ifndef M416_LOAN_OBSERVER_H
#define M416_LOAN_OBSERVER_H
#define zryna_rt_o1_string_from_utf8_copy __real_zryna_rt_o1_string_from_utf8_copy
#define zryna_rt_o1_string_clone __real_zryna_rt_o1_string_clone
#define zryna_rt_o1_string_release __real_zryna_rt_o1_string_release
#include "zryna_ownership_runtime_v1.h"
#undef zryna_rt_o1_string_from_utf8_copy
#undef zryna_rt_o1_string_clone
#undef zryna_rt_o1_string_release
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
typedef struct { uintptr_t pointer, base; uint64_t bytes, length; unsigned live; } acquired;
typedef struct { uintptr_t base; size_t bytes, owner; unsigned live; } physical;
static acquired owners[4];
static physical blocks[4];
static size_t successes, host_successes, attempts, string_clones, string_creations, zero_checks;
static size_t fail_ordinal;
static unsigned inside_runtime;
static const char *fault, *case_name;
static FILE *trace;
static const zryna_rt_o1_handle *tracked_root;
static void record(const char *event,const char *kind,size_t id,uintptr_t pointer,uintptr_t base,uintptr_t source,const uint8_t *bytes,uint64_t length) {
  assert(length<=32);
  assert(fprintf(trace,"{\"event\":\"%s\",\"kind\":\"%s\",\"id\":%zu,\"pointer\":\"%" PRIxPTR "\",\"base\":\"%" PRIxPTR "\",\"source\":\"%" PRIxPTR "\",\"length\":%" PRIu64 ",\"bytes_hex\":\"",event,kind,id,pointer,base,source,length)>0);
  for(uint64_t i=0;i<length;i++) assert(fprintf(trace,"%02x",(unsigned)bytes[i])==2);
  assert(fputs("\"}\n",trace)>=0);
}
static acquired *find(uintptr_t pointer) {
  for(size_t i=0;i<successes;i++) if(owners[i].live && owners[i].pointer==pointer) return &owners[i];
  assert(!"unknown/released String allocation");return NULL;
}
static size_t identity(const acquired *owner) { return (size_t)(owner-owners)+1; }
static void zero_out(const zryna_rt_o1_handle *out,const char *kind) {
  assert(out && out->pointer==0 && out->length==0 && out->capacity==0);
  record("out-zero",kind,++zero_checks,(uintptr_t)out,0,0,NULL,0);
}
static void root_check(size_t id,const char *event) {
  assert(tracked_root && id>0 && id<=successes && owners[id-1].live);
  assert(tracked_root->pointer==owners[id-1].pointer && tracked_root->length==owners[id-1].length && tracked_root->capacity==owners[id-1].bytes);
  record(event,"String",id,tracked_root->pointer,(uintptr_t)tracked_root,0,(const uint8_t *)tracked_root->pointer,tracked_root->length);
}
static void acquire(const zryna_rt_o1_handle *out,uintptr_t source) {
  assert(out->pointer && out->length<=out->capacity && successes<4 && host_successes==successes+1);
  for(size_t i=0;i<successes;i++) assert(!owners[i].live || owners[i].pointer!=out->pointer);
  physical *block=&blocks[host_successes-1];
  assert(block->live && !block->owner && out->pointer>=block->base && out->pointer-block->base<=block->bytes && out->capacity<=block->bytes-(out->pointer-block->base));
  owners[successes]=(acquired){out->pointer,block->base,out->capacity,out->length,1};
  block->owner=++successes;
  record("acquire","String",successes,out->pointer,block->base,source,(const uint8_t *)out->pointer,out->length);
}
#endif
