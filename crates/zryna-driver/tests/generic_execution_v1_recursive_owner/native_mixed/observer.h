#ifndef M416_NATIVE_MIXED_OBSERVER_H
#define M416_NATIVE_MIXED_OBSERVER_H
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
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/resource.h>
#include "private_binding.h"
#include <inttypes.h>
extern int32_t zryna_v1_e_entry(void);
extern void *__real_malloc(size_t);
extern void __real_free(void *);
extern volatile uint8_t *arena;
extern unsigned active_role,inside_runtime;
extern size_t live,attempts,successes,releases,string_clones;
extern unsigned fault_kind,fault_ordinal,fault_hits;
extern uintptr_t pending_last;
uint64_t physical_id(uintptr_t,uint64_t);
int physical_live(uint64_t);
uint64_t known(uintptr_t,unsigned);
uint64_t check_string(const zryna_rt_o1_handle *);
void publish_string(const zryna_rt_o1_handle *,uintptr_t);
void publish_storage(uintptr_t,uint64_t,unsigned);
void release_string(const zryna_rt_o1_handle *);
void string_failure(unsigned,unsigned,uintptr_t,const uint8_t *,uint64_t);
void count_record(const char *,uintptr_t,uint32_t,uint32_t,uint32_t,uint32_t,uint32_t);
void json_start(const char *);
void json_finish(void);
void event(uint64_t,uint64_t,uint64_t,uint64_t);
unsigned enter(unsigned);
void leave(unsigned,uint32_t);
#endif
