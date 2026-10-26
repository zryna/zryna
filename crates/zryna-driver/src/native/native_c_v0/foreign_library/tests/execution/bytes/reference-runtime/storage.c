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
static const uint64_t MAX_DYNAMIC_ALLOCATION_BYTES = UINT64_C(2147483647);
static const uint64_t TARGET_ALLOCATION_BUDGET = UINT64_C(67108864);
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
  if (byte_size > MAX_DYNAMIC_ALLOCATION_BYTES) return RT_CAPACITY;
  if (byte_size > TARGET_ALLOCATION_BUDGET) return RT_ALLOCATION;
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

