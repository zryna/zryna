#ifndef ZRYNA_OWNERSHIP_RUNTIME_V1_H
#define ZRYNA_OWNERSHIP_RUNTIME_V1_H

#include <stdint.h>

typedef struct {
  uintptr_t pointer;
  uint64_t length;
  uint64_t capacity;
} zryna_rt_o1_handle;

uint32_t zryna_rt_o1_allocate(uint64_t byte_size, uint32_t alignment,
                              uintptr_t *out_pointer);
uint32_t zryna_rt_o1_grow(uintptr_t pointer, uint64_t old_byte_size,
                          uint64_t new_byte_size, uint32_t alignment,
                          uintptr_t *out_pointer);
uint32_t zryna_rt_o1_release(uintptr_t pointer, uint64_t byte_size,
                             uint32_t alignment);
uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes, uint64_t byte_length,
                                           zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,
                                  zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,
                                   const zryna_rt_o1_handle *right,
                                   zryna_rt_o1_handle *out_string);
uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value);
uint32_t zryna_rt_o1_vec_allocate(uint32_t element_layout_id, uint64_t required_capacity,
                                  zryna_rt_o1_handle *out_storage);
uint32_t zryna_rt_o1_vec_reserve(uint32_t element_layout_id,
                                 const zryna_rt_o1_handle *storage,
                                 uint64_t required_length,
                                 zryna_rt_o1_handle *out_storage);
uint32_t zryna_rt_o1_vec_release_storage(uint32_t element_layout_id,
                                         const zryna_rt_o1_handle *storage);
uint32_t zryna_rt_o1_strong_clone(uintptr_t control);
uint32_t zryna_rt_o1_weak_downgrade(uintptr_t control);
uint32_t zryna_rt_o1_weak_clone(uintptr_t control);
uint32_t zryna_rt_o1_weak_upgrade(uintptr_t control);
uint32_t zryna_rt_o1_strong_release_begin(uintptr_t control,
                                          uint32_t *out_is_last_strong);
uint32_t zryna_rt_o1_strong_release_finish(uintptr_t control);
uint32_t zryna_rt_o1_weak_release(uintptr_t control, uint32_t *out_deallocated);

#endif
