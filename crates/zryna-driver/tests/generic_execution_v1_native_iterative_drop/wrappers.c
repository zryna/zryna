/* Real canonical calls only; test wrappers do not implement allocation/control semantics. */
#include "observer.h"
uint32_t __wrap_zryna_rt_o1_allocate(uint64_t bytes,uint32_t alignment,uintptr_t *out) {
  assert(out && *out==0);
  unsigned previous=enter(0);
  uint32_t status=__real_zryna_rt_o1_allocate(bytes,alignment,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_grow(uintptr_t pointer,uint64_t old_bytes,uint64_t new_bytes,uint32_t alignment,uintptr_t *out) {
  assert(out && *out==0);
  unsigned previous=enter(1);
  uint32_t status=__real_zryna_rt_o1_grow(pointer,old_bytes,new_bytes,alignment,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_release(uintptr_t pointer,uint64_t bytes,uint32_t alignment) {
  unsigned previous=enter(2);
  uint32_t status=__real_zryna_rt_o1_release(pointer,bytes,alignment);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
  unsigned previous=enter(3);
  uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
  unsigned previous=enter(4);
  zryna_rt_o1_handle before=*source;string_clones++;
  if(fault_clone && string_clones==1) {
    *out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX};
    event(6,4,string_clones,1);assert(memcmp(source,&before,sizeof(before))==0);
    leave(previous,1);return 1;
  }
  uint32_t status=__real_zryna_rt_o1_string_clone(source,out);
  assert(memcmp(source,&before,sizeof(before))==0);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,const zryna_rt_o1_handle *right,zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
  unsigned previous=enter(5);
  uint32_t status=__real_zryna_rt_o1_string_concat(left,right,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  unsigned previous=enter(6);
  uint32_t status=__real_zryna_rt_o1_string_release(value);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_allocate(uint32_t element,uint64_t capacity,zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
  unsigned previous=enter(7);
  uint32_t status=__real_zryna_rt_o1_vec_allocate(element,capacity,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_reserve(uint32_t element,const zryna_rt_o1_handle *source,uint64_t length,zryna_rt_o1_handle *out) {
  assert(out && !out->pointer && !out->length && !out->capacity);
  unsigned previous=enter(8);
  uint32_t status=__real_zryna_rt_o1_vec_reserve(element,source,length,out);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_vec_release_storage(uint32_t element,const zryna_rt_o1_handle *value) {
  unsigned previous=enter(9);
  uint32_t status=__real_zryna_rt_o1_vec_release_storage(element,value);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t control) {
  unsigned previous=enter(10);
  uint32_t status=__real_zryna_rt_o1_strong_clone(control);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t control) {
  unsigned previous=enter(11);
  uint32_t status=__real_zryna_rt_o1_weak_downgrade(control);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t control) {
  unsigned previous=enter(12);
  uint32_t status=__real_zryna_rt_o1_weak_clone(control);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t control) {
  unsigned previous=enter(13);
  uint32_t status=__real_zryna_rt_o1_weak_upgrade(control);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t control,uint32_t *out) {
  assert(out && *out==0);
  unsigned previous=enter(14);
  uint32_t status=__real_zryna_rt_o1_strong_release_begin(control,out);
  event(6,14,*out,status);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t control) {
  unsigned previous=enter(15);
  uint32_t status=__real_zryna_rt_o1_strong_release_finish(control);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t control,uint32_t *out) {
  assert(out && *out==0);
  unsigned previous=enter(16);
  uint32_t status=__real_zryna_rt_o1_weak_release(control,out);
  event(6,16,*out,status);
  leave(previous,status);return status;
}
