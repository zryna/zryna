/* Private observer wraps the unchanged canonical17 ABI; no replacement runtime. */
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
#include "runtime_metadata.h"
#include "entry_abi.h"
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
typedef struct { uintptr_t base,pointer; size_t bytes; uint64_t payload_bytes,length; unsigned kind,live,released; } owner;
static owner owners[5];
static size_t successes,attempts,fail_ordinal;
static FILE *trace;
static void hex(const uint8_t *bytes,uint64_t length) {
  assert(length<=64);for(uint64_t i=0;i<length;i++) assert(fprintf(trace,"%02x",(unsigned)bytes[i])==2);
}
static owner *find(uintptr_t pointer,unsigned kind) {
  for(size_t i=0;i<successes;i++) if(owners[i].pointer==pointer && owners[i].live) {assert(owners[i].kind==kind && !owners[i].released);return &owners[i];}
  assert(!"unknown or already released allocation");return NULL;
}
static size_t identity(const owner *value) {return (size_t)(value-owners)+1;}
static void zero(const zryna_rt_o1_handle *out) {assert(out && !out->pointer && !out->length && !out->capacity);}
static void poison(zryna_rt_o1_handle *out) {*out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX};}
static void completed(uint32_t status,zryna_rt_o1_handle *out) {
  assert(status==0 || status==1);if(status) {zero(out);assert(fputs("{\"event\":\"failed-output\",\"canonical_zero\":true,\"poisoned_after_return\":true}\n",trace)>=0);poison(out);}
}
