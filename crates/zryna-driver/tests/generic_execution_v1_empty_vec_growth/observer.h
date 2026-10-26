#define zryna_rt_o1_vec_allocate __real_zryna_rt_o1_vec_allocate
#define zryna_rt_o1_vec_reserve __real_zryna_rt_o1_vec_reserve
#define zryna_rt_o1_vec_release_storage __real_zryna_rt_o1_vec_release_storage
#include "zryna_ownership_runtime_v1.h"
#undef zryna_rt_o1_vec_allocate
#undef zryna_rt_o1_vec_reserve
#undef zryna_rt_o1_vec_release_storage
#include "runtime_metadata.h"
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
extern void *__real_malloc(size_t size);
extern void __real_free(void *pointer);
static FILE *trace;
static size_t attempts, acquired, released, reserves, fail_ordinal, block_bytes;
static uintptr_t block_base, payload;
static int fault_fired, releasing;
static void event(const char *name, size_t id) {
  assert(fprintf(trace,"{\"event\":\"%s\",\"id\":%zu}\n",name,id)>0);
}
static void zero_out(zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
}
static void layout(uint32_t id) {
  uint64_t stride=0;uint32_t alignment=0;
  assert(metadata_element(id,&stride,&alignment) && stride==4 && alignment==4);
}
