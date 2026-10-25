/* Real canonical calls only; test wrappers do not implement allocation/control semantics. */
#include "mixed-observer.h"
#include "runtime_metadata.h"
#include "native_payloads.h"
#include "mixed-alloc.h"
#include "mixed-counts.h"
uint32_t __wrap_zryna_rt_o1_grow(uintptr_t p,uint64_t a,uint64_t z,uint32_t n,uintptr_t *o) {
  unsigned previous=enter(1);uint32_t status=__real_zryna_rt_o1_grow(p,a,z,n,o);leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_release(uintptr_t p,uint64_t n,uint32_t a) {
  unsigned previous=enter(2);uint32_t status=__real_zryna_rt_o1_release(p,n,a);leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *a,const zryna_rt_o1_handle *z,zryna_rt_o1_handle *o) {
  unsigned previous=enter(5);uint32_t status=__real_zryna_rt_o1_string_concat(a,z,o);leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t e,const zryna_rt_o1_handle *s,uint64_t n,zryna_rt_o1_handle *o) {
  unsigned previous=enter(8);uint32_t status=__real_zryna_rt_o1_vec_reserve(e,s,n,o);leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t c) {
  unsigned previous=enter(13);uint32_t status=__real_zryna_rt_o1_weak_upgrade(c);leave(previous,status);return status;
}
