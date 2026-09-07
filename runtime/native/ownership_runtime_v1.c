#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uintptr_t pointer;
  uint64_t length;
  uint64_t capacity;
} zryna_rt_o1_handle;

enum {
  RT_OK = 0,
  RT_ALLOCATION = 1,
  RT_CAPACITY = 2,
  RT_REFCOUNT = 3,
  RT_UTF8 = 4,
  RT_EXPIRED = 5,
  RT_ABI = 255
};

typedef struct allocation_header_tag {
  uint64_t magic;
  uint64_t size;
  uintptr_t base;
  uintptr_t data;
  struct allocation_header_tag *previous;
  struct allocation_header_tag *next;
  uint32_t alignment;
  uint32_t pending_last_strong;
#ifdef ZRYNA_M3_OBSERVATION
  uint32_t scratch;
#endif
} allocation_header;

static const uint64_t ALLOCATION_MAGIC = UINT64_C(0x7a72796e616f3175);
static const uint64_t MAX_ALLOCATION_BYTES = UINT64_C(67108864);
static const uint64_t MAX_VEC_ELEMENTS = UINT64_C(1048576);
static allocation_header *allocation_head = NULL;
#ifdef ZRYNA_M3_OBSERVATION
extern uint32_t zryna_m3_observe(uint32_t command);
#endif
#ifdef ZRYNA_RT_O1_FAIL_ALLOCATION_AT
static uint64_t allocation_attempt = 0;
#endif

static int valid_alignment(uint32_t alignment) {
  return alignment != 0 && alignment <= 32768U &&
         (alignment & (alignment - 1U)) == 0;
}

static allocation_header *header_for(uintptr_t pointer) {
  allocation_header *header = allocation_head;
  if (pointer == 0) {
    return NULL;
  }
  while (header != NULL) {
    if (header->data == pointer) {
      return header;
    }
    header = header->next;
  }
  return NULL;
}

static int valid_string_storage(const zryna_rt_o1_handle *value) {
  allocation_header *header;
  if (value == NULL || value->length > value->capacity) {
    return 0;
  }
  if (value->capacity == 0) {
    return value->pointer == 0;
  }
  header = header_for(value->pointer);
  return header != NULL && header->size == value->capacity &&
         header->alignment == 1 && header->pending_last_strong == 0;
}

static int valid_control_storage(const allocation_header *header) {
  return header != NULL && header->size >= 2U * sizeof(uint32_t) &&
         header->alignment >= _Alignof(uint32_t);
}

static uint32_t allocate_bytes(uint64_t byte_size, uint32_t alignment,
                               uintptr_t *out_pointer) {
  size_t total;
  uintptr_t unaligned;
  uintptr_t aligned;
  void *base;
  allocation_header *header;
  if (out_pointer == NULL) {
    return RT_ABI;
  }
  *out_pointer = 0;
  if (!valid_alignment(alignment)) {
    return RT_ABI;
  }
  if (byte_size == 0) {
    return RT_OK;
  }
  if (byte_size > MAX_ALLOCATION_BYTES || byte_size > SIZE_MAX ||
      (size_t)byte_size > SIZE_MAX - sizeof(allocation_header) - alignment) {
    return RT_CAPACITY;
  }
#ifdef ZRYNA_M3_OBSERVATION
  if (zryna_m3_observe(UINT32_C(0x10000006)) != 0) return RT_ALLOCATION;
#endif
#ifdef ZRYNA_RT_O1_FAIL_ALLOCATIONS
  return RT_ALLOCATION;
#else
#ifdef ZRYNA_RT_O1_FAIL_ALLOCATION_AT
  ++allocation_attempt;
  if (allocation_attempt == ZRYNA_RT_O1_FAIL_ALLOCATION_AT) {
    return RT_ALLOCATION;
  }
#endif
  total = (size_t)byte_size + sizeof(allocation_header) + alignment - 1U;
  base = malloc(total);
  if (base == NULL) {
    return RT_ALLOCATION;
  }
  unaligned = (uintptr_t)base + sizeof(allocation_header);
  aligned = (unaligned + (uintptr_t)alignment - 1U) &
            ~((uintptr_t)alignment - 1U);
  header = (allocation_header *)(aligned - sizeof(allocation_header));
  header->magic = ALLOCATION_MAGIC;
  header->size = byte_size;
  header->base = (uintptr_t)base;
  header->data = aligned;
  header->previous = NULL;
  header->next = allocation_head;
  if (allocation_head != NULL) {
    allocation_head->previous = header;
  }
  allocation_head = header;
  header->alignment = alignment;
  header->pending_last_strong = 0;
#ifdef ZRYNA_M3_OBSERVATION
  header->scratch = 0;
#endif
  *out_pointer = aligned;
  return RT_OK;
#endif
}

static uint32_t release_known(allocation_header *header) {
  void *base;
  if (header == NULL || header->pending_last_strong != 0) {
    return RT_ABI;
  }
  base = (void *)header->base;
  if (header->previous != NULL) {
    header->previous->next = header->next;
  } else {
    allocation_head = header->next;
  }
  if (header->next != NULL) {
    header->next->previous = header->previous;
  }
  header->magic = 0;
  header->base = 0;
  header->data = 0;
  free(base);
  return RT_OK;
}

uint32_t zryna_rt_o1_allocate(uint64_t byte_size, uint32_t alignment,
                              uintptr_t *out_pointer) {
  return allocate_bytes(byte_size, alignment, out_pointer);
}

uint32_t zryna_rt_o1_grow(uintptr_t pointer, uint64_t old_byte_size,
                          uint64_t new_byte_size, uint32_t alignment,
                          uintptr_t *out_pointer) {
  allocation_header *old_header;
  uintptr_t replacement;
  uint32_t status;
  if (out_pointer == NULL) {
    return RT_ABI;
  }
  *out_pointer = 0;
  if (old_byte_size == 0 && pointer == 0) {
    return allocate_bytes(new_byte_size, alignment, out_pointer);
  }
  old_header = header_for(pointer);
  if (old_header == NULL || old_header->size != old_byte_size ||
      old_header->alignment != alignment || old_header->pending_last_strong != 0) {
    return RT_ABI;
  }
  if (new_byte_size == old_byte_size) {
    *out_pointer = pointer;
    return RT_OK;
  }
  if (new_byte_size == 0) {
    return release_known(old_header);
  }
  status = allocate_bytes(new_byte_size, alignment, &replacement);
  if (status != RT_OK) {
    return status;
  }
  memcpy((void *)replacement, (const void *)pointer,
         (size_t)(old_byte_size < new_byte_size ? old_byte_size : new_byte_size));
  status = release_known(old_header);
  if (status != RT_OK) {
    (void)release_known(header_for(replacement));
    return status;
  }
  *out_pointer = replacement;
  return RT_OK;
}

uint32_t zryna_rt_o1_release(uintptr_t pointer, uint64_t byte_size,
                             uint32_t alignment) {
  allocation_header *header;
  if (pointer == 0 && byte_size == 0 && valid_alignment(alignment)) {
    return RT_OK;
  }
  header = header_for(pointer);
  if (header == NULL || header->size != byte_size ||
      header->alignment != alignment) {
    return RT_ABI;
  }
  return release_known(header);
}

static int valid_utf8(const uint8_t *bytes, uint64_t length) {
  uint64_t index = 0;
  while (index < length) {
    uint8_t first = bytes[index++];
    uint32_t code;
    uint32_t minimum;
    uint32_t remaining;
    if (first < 0x80U) {
      continue;
    }
    if (first >= 0xc2U && first <= 0xdfU) {
      code = first & 0x1fU;
      minimum = 0x80U;
      remaining = 1;
    } else if (first >= 0xe0U && first <= 0xefU) {
      code = first & 0x0fU;
      minimum = 0x800U;
      remaining = 2;
    } else if (first >= 0xf0U && first <= 0xf4U) {
      code = first & 0x07U;
      minimum = 0x10000U;
      remaining = 3;
    } else {
      return 0;
    }
    if (length - index < remaining) {
      return 0;
    }
    while (remaining-- != 0) {
      uint8_t next = bytes[index++];
      if ((next & 0xc0U) != 0x80U) {
        return 0;
      }
      code = (code << 6) | (next & 0x3fU);
    }
    if (code < minimum || code > 0x10ffffU ||
        (code >= 0xd800U && code <= 0xdfffU)) {
      return 0;
    }
  }
  return 1;
}

uint32_t zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,
                                            uint64_t byte_length,
                                            zryna_rt_o1_handle *out_string) {
  uintptr_t pointer;
  uint32_t status;
  if (out_string == NULL || (bytes == NULL && byte_length != 0)) {
    return RT_ABI;
  }
  memset(out_string, 0, sizeof(*out_string));
  if (!valid_utf8(bytes, byte_length)) {
    return RT_UTF8;
  }
  status = allocate_bytes(byte_length, 1, &pointer);
  if (status != RT_OK) {
    return status;
  }
  if (byte_length != 0) {
    memcpy((void *)pointer, bytes, (size_t)byte_length);
  }
  out_string->pointer = pointer;
  out_string->length = byte_length;
  out_string->capacity = byte_length;
  return RT_OK;
}

uint32_t zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,
                                  zryna_rt_o1_handle *out_string) {
  if (out_string == source) {
    return RT_ABI;
  }
  if (!valid_string_storage(source)) {
    if (out_string != NULL) {
      memset(out_string, 0, sizeof(*out_string));
    }
    return RT_ABI;
  }
  return zryna_rt_o1_string_from_utf8_copy((const uint8_t *)source->pointer,
                                            source->length, out_string);
}

uint32_t zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,
                                   const zryna_rt_o1_handle *right,
                                   zryna_rt_o1_handle *out_string) {
  uint64_t length;
  uintptr_t pointer;
  uint32_t status;
  if (out_string == left || out_string == right) {
    return RT_ABI;
  }
  if (out_string == NULL || !valid_string_storage(left) ||
      !valid_string_storage(right)) {
    if (out_string != NULL) {
      memset(out_string, 0, sizeof(*out_string));
    }
    return RT_ABI;
  }
  memset(out_string, 0, sizeof(*out_string));
  if (left->length > MAX_ALLOCATION_BYTES - right->length) {
    return RT_CAPACITY;
  }
  length = left->length + right->length;
  status = allocate_bytes(length, 1, &pointer);
  if (status != RT_OK) {
    return status;
  }
  if (left->length != 0) {
    memcpy((void *)pointer, (const void *)left->pointer, (size_t)left->length);
  }
  if (right->length != 0) {
    memcpy((void *)(pointer + left->length), (const void *)right->pointer,
           (size_t)right->length);
  }
  out_string->pointer = pointer;
  out_string->length = length;
  out_string->capacity = length;
  return RT_OK;
}

uint32_t zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  if (value == NULL || value->length > value->capacity) {
    return RT_ABI;
  }
  return zryna_rt_o1_release(value->pointer, value->capacity, 1);
}

static int vec_layout(uint32_t id, uint64_t *stride, uint32_t *alignment) {
  (void)stride;
  (void)alignment;
  switch (id) {
  /* ZRYNA_RT_O1_ELEMENT_LAYOUT_CASES */
  default:
    return 0;
  }
}

static int valid_vec_storage(const zryna_rt_o1_handle *storage,
                             uint64_t stride, uint32_t alignment) {
  allocation_header *header;
  if (storage == NULL || stride == 0 || storage->length > storage->capacity ||
      storage->capacity > MAX_VEC_ELEMENTS ||
      storage->capacity > MAX_ALLOCATION_BYTES / stride) {
    return 0;
  }
  if (storage->capacity == 0) {
    return storage->pointer == 0;
  }
  header = header_for(storage->pointer);
  return header != NULL && header->size == storage->capacity * stride &&
         header->alignment == alignment && header->pending_last_strong == 0;
}

uint32_t zryna_rt_o1_vec_allocate(uint32_t element_layout_id,
                                  uint64_t required_capacity,
                                  zryna_rt_o1_handle *out_storage) {
  uint64_t stride;
  uint32_t alignment;
  uintptr_t pointer;
  uint32_t status;
  if (out_storage == NULL) {
    return RT_ABI;
  }
  memset(out_storage, 0, sizeof(*out_storage));
  if (!vec_layout(element_layout_id, &stride, &alignment)) {
    return RT_ABI;
  }
  if (required_capacity > MAX_VEC_ELEMENTS ||
      required_capacity > MAX_ALLOCATION_BYTES / stride) {
    return RT_CAPACITY;
  }
  status = allocate_bytes(required_capacity * stride, alignment, &pointer);
  if (status != RT_OK) {
    return status;
  }
  out_storage->pointer = pointer;
  out_storage->capacity = required_capacity;
  return RT_OK;
}

uint32_t zryna_rt_o1_vec_reserve(uint32_t element_layout_id,
                                 const zryna_rt_o1_handle *storage,
                                 uint64_t required_length,
                                 zryna_rt_o1_handle *out_storage) {
  uint64_t stride;
  uint64_t capacity;
  uint32_t alignment;
  uintptr_t pointer;
  uint32_t status;
  if (storage == out_storage) {
    return RT_ABI;
  }
  if (out_storage == NULL || !vec_layout(element_layout_id, &stride, &alignment) ||
      !valid_vec_storage(storage, stride, alignment) ||
      required_length < storage->length) {
    if (out_storage != NULL) {
      memset(out_storage, 0, sizeof(*out_storage));
    }
    return RT_ABI;
  }
  memset(out_storage, 0, sizeof(*out_storage));
  if (required_length <= storage->capacity) {
    *out_storage = *storage;
    return RT_OK;
  }
  capacity = storage->capacity == 0 ? 1 : storage->capacity;
  while (capacity < required_length && capacity <= MAX_VEC_ELEMENTS / 2) {
    capacity *= 2;
  }
  if (capacity < required_length) {
    capacity = required_length;
  }
  if (capacity > MAX_VEC_ELEMENTS || capacity > MAX_ALLOCATION_BYTES / stride) {
    return RT_CAPACITY;
  }
  status = zryna_rt_o1_grow(storage->pointer, storage->capacity * stride,
                            capacity * stride, alignment, &pointer);
  if (status != RT_OK) {
    return status;
  }
  out_storage->pointer = pointer;
  out_storage->length = storage->length;
  out_storage->capacity = capacity;
  return RT_OK;
}

uint32_t zryna_rt_o1_vec_release_storage(uint32_t element_layout_id,
                                         const zryna_rt_o1_handle *storage) {
  uint64_t stride;
  uint32_t alignment;
  if (!vec_layout(element_layout_id, &stride, &alignment) ||
      !valid_vec_storage(storage, stride, alignment)) {
    return RT_ABI;
  }
  return zryna_rt_o1_release(storage->pointer, storage->capacity * stride,
                             alignment);
}

uint32_t zryna_rt_o1_strong_clone(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      *strong == 0) {
    return RT_ABI;
  }
  if (*strong == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++*strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_downgrade(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      counts[0] == 0) {
    return RT_ABI;
  }
  if (counts[1] == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++counts[1];
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_clone(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0 ||
      counts[1] == 0) {
    return RT_ABI;
  }
  if (counts[1] == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++counts[1];
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_upgrade(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong != 0) {
    return RT_ABI;
  }
  if (*strong == 0) {
    return RT_EXPIRED;
  }
  if (*strong == UINT32_MAX) {
    return RT_REFCOUNT;
  }
  ++*strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_strong_release_begin(uintptr_t control,
                                          uint32_t *out_is_last_strong) {
  allocation_header *header = header_for(control);
  uint32_t *strong = (uint32_t *)control;
  if (out_is_last_strong == NULL || !valid_control_storage(header) ||
      header->pending_last_strong != 0 || *strong == 0) {
    return RT_ABI;
  }
  --*strong;
  *out_is_last_strong = (uint32_t)(*strong == 0);
  header->pending_last_strong = *out_is_last_strong;
  return RT_OK;
}

uint32_t zryna_rt_o1_strong_release_finish(uintptr_t control) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (!valid_control_storage(header) || header->pending_last_strong == 0 ||
      counts[0] != 0 || counts[1] == 0) {
    return RT_ABI;
  }
  --counts[1];
  header->pending_last_strong = 0;
  if (counts[1] == 0) {
    return release_known(header);
  }
  return RT_OK;
}

uint32_t zryna_rt_o1_weak_release(uintptr_t control,
                                  uint32_t *out_deallocated) {
  allocation_header *header = header_for(control);
  uint32_t *counts = (uint32_t *)control;
  if (out_deallocated == NULL || !valid_control_storage(header) ||
      header->pending_last_strong != 0 || counts[1] == 0 ||
      (counts[0] != 0 && counts[1] == 1)) {
    return RT_ABI;
  }
  --counts[1];
  *out_deallocated = (uint32_t)(counts[1] == 0);
  if (*out_deallocated != 0) {
    return release_known(header);
  }
  return RT_OK;
}

#ifdef ZRYNA_M3_OBSERVATION
/* Non-owning SSA descriptors live until the scalar invocation has completed. */
uint32_t zryna_m3_allocate_record(uint64_t size, uint32_t alignment, uintptr_t *out) {
  uint32_t status = allocate_bytes(size, alignment, out);
  if (status == RT_OK && *out != 0) {
    allocation_header *header = header_for(*out);
    if (header == NULL) return RT_ABI;
    header->scratch = 1;
  }
  return status;
}
uint32_t zryna_m3_finish_invocation(void) {
  allocation_header *current = allocation_head;
  while (current != NULL) {
    allocation_header *next = current->next;
    if (current->scratch && release_known(current) != RT_OK) return RT_ABI;
    current = next;
  }
  /* Owned payloads/control blocks must have been released by generated cleanup. */
  return allocation_head == NULL ? RT_OK : RT_ABI;
}
#endif
