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
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
typedef struct { uintptr_t pointer; uint64_t bytes; unsigned kind, live, released; } acquired;
static acquired owners[6];
static size_t successes, attempts, string_clones, count_attempts[3];
static size_t fail_ordinal;
static const char *fault, *case_name;
static FILE *trace;
static uintptr_t pending_last;
static acquired *find(uintptr_t pointer, unsigned kind) {
  for (size_t i=0;i<successes;i++) if(owners[i].pointer==pointer && owners[i].live) {
    assert(owners[i].kind==kind && owners[i].released==0); return &owners[i];
  }
  assert(!"unknown or already released physical allocation"); return NULL;
}
static void record(const char *event, const char *kind, size_t id, uintptr_t pointer, uintptr_t source, const uint8_t *bytes, uint64_t length) {
  assert(length<65536);
  assert(fprintf(trace,"{\"event\":\"%s\",\"kind\":\"%s\",\"id\":%zu,\"pointer\":\"%" PRIxPTR "\",\"source\":\"%" PRIxPTR "\",\"length\":%" PRIu64 ",\"bytes_hex\":\"",event,kind,id,pointer,source,length)>0);
  for(uint64_t i=0;i<length;i++) assert(fprintf(trace,"%02x",(unsigned)bytes[i])==2);
  assert(fputs("\"}\n",trace)>=0);
}
static const char *kind_name(unsigned kind) { return kind==1 ? "String" : kind==2 ? "Vec" : "control"; }
static void acquire(uintptr_t pointer, uint64_t bytes, unsigned kind, uintptr_t source, uint64_t string_length) {
  assert(pointer && successes<6);
  for(size_t i=0;i<successes;i++) assert(owners[i].pointer!=pointer);
  owners[successes++]=(acquired){pointer,bytes,kind,1,0};
  record("acquire",kind_name(kind),successes,pointer,source,kind==1 ? (const uint8_t *)pointer : NULL,string_length);
}
static void releasing(acquired *owner, const uint8_t *bytes, uint64_t length) {
  record("release",kind_name(owner->kind),(size_t)(owner-owners)+1,owner->pointer,0,bytes,length);
  owner->live=0; owner->released++;
}
static int allocation_failure(uintptr_t source, const uint8_t *bytes, uint64_t length) {
  attempts++;
  if(strcmp(fault,"allocation")==0 && attempts==fail_ordinal) {
    record("failure","allocation",attempts,0,source,bytes,length); return 1;
  }
  return 0;
}
static void poison(zryna_rt_o1_handle *out) { assert(out && out->pointer==0 && out->length==0 && out->capacity==0); *out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX}; }
static void count_event(const char *name, uintptr_t pointer, uint32_t before_s, uint32_t before_w, uint32_t after_s, uint32_t after_w, uint32_t status) {
  assert(fprintf(trace,"{\"event\":\"count\",\"name\":\"%s\",\"pointer\":\"%" PRIxPTR "\",\"beforeStrong\":%u,\"beforeWeak\":%u,\"afterStrong\":%u,\"afterWeak\":%u,\"status\":%u}\n",name,pointer,before_s,before_w,after_s,after_w,status)>0);
}
