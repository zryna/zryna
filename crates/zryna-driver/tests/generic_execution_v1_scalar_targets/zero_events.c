/* Test-only observation: no replacement allocator/status/provider is supplied. */
#include "zryna_ownership_runtime_v1.h"
#include <stdlib.h>
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t n,uint32_t a,uintptr_t *out) { (void)n;(void)a;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_grow(uintptr_t p,uint64_t old,uint64_t n,uint32_t a,uintptr_t *out) { (void)p;(void)old;(void)n;(void)a;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_release(uintptr_t p,uint64_t n,uint32_t a) { (void)p;(void)n;(void)a; abort(); }
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *p,uint64_t n,zryna_rt_o1_handle *out) { (void)p;(void)n;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *p,zryna_rt_o1_handle *out) { (void)p;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *a,const zryna_rt_o1_handle *b,zryna_rt_o1_handle *out) { (void)a;(void)b;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t e,uint64_t n,zryna_rt_o1_handle *out) { (void)e;(void)n;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t e,const zryna_rt_o1_handle *p,uint64_t n,zryna_rt_o1_handle *out) { (void)e;(void)p;(void)n;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t e,const zryna_rt_o1_handle *p) { (void)e;(void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t p) { (void)p; abort(); }
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t p,uint32_t *out) { (void)p;(void)out; abort(); }
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t p,uint32_t *out) { (void)p;(void)out; abort(); }
